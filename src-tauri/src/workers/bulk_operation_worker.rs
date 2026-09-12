use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use futures::{stream, FutureExt};
use tauri::async_runtime::{channel, JoinHandle, Mutex, Sender};

use crate::error::{AppError, AppResult};
use crate::models::bulk_operation_model::{BulkOperation, BulkOperationJob};
use crate::models::delete_model::DeleteFileJob;
use crate::models::file_processing_model::{FileProcessingJob, UpdateFileMetadataJob};
use crate::repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
use crate::repositories::database_repository::SqliteDatabase;
use crate::services::bulk_operation_service::BulkOperationService;
use crate::utils::constants::{
    BULK_OPERATION_BATCH_SIZE, BULK_OPERATION_QUEUE, BULK_OPERATION_WORKER, FILE_PROCESSING_QUEUE,
};
use crate::utils::operation_logger::log_event;

pub struct BulkOperationWorker {
    shutdown: Sender<()>,
    task: Mutex<Option<JoinHandle<Result<(), WorkerError>>>>,
}

impl BulkOperationWorker {
    pub fn start(database: &SqliteDatabase, bulk_operations: BulkOperationService) -> Self {
        let queue_pool = database.pool().clone();
        let repository = SqliteBackgroundProcessingRepository::new(database.clone());
        let (shutdown, mut shutdown_receiver) = channel(1);
        let shutdown_signal = async move {
            let _ = shutdown_receiver.recv().await;
        }
        .boxed()
        .shared();
        let task = tauri::async_runtime::spawn(async move {
            log_event(BULK_OPERATION_WORKER, "START", "worker supervisor started");
            loop {
                let backend = SqliteStorage::<BulkOperationJob, (), ()>::new_with_config(
                    &queue_pool,
                    &super::queue_config(BULK_OPERATION_QUEUE),
                );
                let handler_pool = queue_pool.clone();
                let handler_bulk_operations = bulk_operations.clone();
                let handler_repository = repository.clone();
                let worker = WorkerBuilder::new(format!(
                    "{}-{}",
                    BULK_OPERATION_WORKER,
                    uuid::Uuid::new_v4()
                ))
                .backend(backend)
                .concurrency(1)
                .build(move |job: BulkOperationJob| {
                    let pool = handler_pool.clone();
                    let bulk_operations = handler_bulk_operations.clone();
                    let repository = handler_repository.clone();
                    async move {
                        let subject = job.subject();
                        log_event(
                            BULK_OPERATION_WORKER,
                            "RECEIVED",
                            format!("process_id={} {subject}", job.process_id),
                        );
                        repository.mark_running(&job.process_id).await?;
                        let process_id = job.process_id.clone();
                        match dispatch_bulk_operation(
                            job,
                            pool,
                            bulk_operations,
                            repository,
                        )
                        .await?
                        {
                            BulkDispatchOutcome::Complete { dispatched } => log_event(
                                BULK_OPERATION_WORKER,
                                "DISPATCHED",
                                format!("process_id={process_id} dispatched={dispatched} child_queue={FILE_PROCESSING_QUEUE}"),
                            ),
                            BulkDispatchOutcome::Failed {
                                dispatched,
                                undispatched,
                                message,
                            } => log_event(
                                BULK_OPERATION_WORKER,
                                "PARTIAL-FAILURE",
                                format!("process_id={process_id} dispatched={dispatched} undispatched={undispatched} error={message}"),
                            ),
                        }
                        Ok::<(), AppError>(())
                    }
                });
                let stop = shutdown_signal.clone();
                let result = worker
                    .run_until(async move {
                        stop.await;
                        Ok::<(), std::io::Error>(())
                    })
                    .await;
                if shutdown_signal.clone().now_or_never().is_some() {
                    log_event(BULK_OPERATION_WORKER, "STOP", "worker supervisor stopped");
                    return result;
                }
                eprintln!("[{BULK_OPERATION_WORKER}][RESTART] worker exited: {result:?}");
                tokio::select! {
                    _ = shutdown_signal.clone() => return Ok(()),
                    _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
                }
            }
        });
        Self {
            shutdown,
            task: Mutex::new(Some(task)),
        }
    }

    pub async fn close(&self) {
        log_event(BULK_OPERATION_WORKER, "STOP", "shutdown requested");
        let _ = self.shutdown.send(()).await;
        if let Some(task) = self.task.lock().await.take() {
            let _ = task.await;
        }
    }
}

enum BulkDispatchOutcome {
    Complete {
        dispatched: usize,
    },
    Failed {
        dispatched: usize,
        undispatched: usize,
        message: String,
    },
}

async fn dispatch_bulk_operation(
    job: BulkOperationJob,
    pool: sqlx::SqlitePool,
    bulk_operations: BulkOperationService,
    repository: SqliteBackgroundProcessingRepository,
) -> AppResult<BulkDispatchOutcome> {
    let expected = usize::try_from(job.expected_count)
        .map_err(|_| AppError::validation("The bulk operation contains too many files."))?;
    let targets = match collect_targets_with_retry(&job, bulk_operations).await {
        Ok(targets) => targets,
        Err(message) => {
            repository
                .fail_remaining_items(&job.process_id, job.expected_count, &message)
                .await?;
            return Ok(BulkDispatchOutcome::Failed {
                dispatched: 0,
                undispatched: expected,
                message,
            });
        }
    };
    let targets = targets.into_iter().take(expected).collect::<Vec<_>>();
    let missing = expected.saturating_sub(targets.len());
    log_event(
        BULK_OPERATION_WORKER,
        "BATCH-MATCHED",
        format!(
            "process_id={} expected={expected} matched={} missing={missing}",
            job.process_id,
            targets.len()
        ),
    );

    if missing > 0 {
        repository
            .rollback_items(&job.process_id, missing as u64)
            .await?;
    }

    let mut dispatched = 0_usize;
    for batch in targets.chunks(BULK_OPERATION_BATCH_SIZE) {
        let batch = batch.to_vec();
        let batch_size = batch.len();
        let process_id = job.process_id.clone();
        let operation = job.operation.clone();
        let queue_pool = pool.clone();
        let batch_subject =
            format!("process_id={process_id} offset={dispatched} items={batch_size}");
        let outcome =
            super::retry::retry_and_ack(BULK_OPERATION_QUEUE, &batch_subject, move || {
                let batch = batch.clone();
                let operation = operation.clone();
                let process_id = process_id.clone();
                let queue_pool = queue_pool.clone();
                async move {
                    let jobs = batch
                        .into_iter()
                        .map(|target| {
                            let child_job = file_processing_job(&operation, &process_id, target);
                            Task::builder(child_job).build()
                        })
                        .collect::<Vec<_>>();
                    let mut jobs = stream::iter(jobs);
                    let mut queue = SqliteStorage::<FileProcessingJob, (), ()>::new_in_queue(
                        &queue_pool,
                        FILE_PROCESSING_QUEUE,
                    );
                    queue.push_all(&mut jobs).await.map_err(AppError::database)
                }
            })
            .await?;
        match outcome {
            super::retry::JobOutcome::Completed => {
                dispatched += batch_size;
                log_event(
                    BULK_OPERATION_WORKER,
                    "BATCH-QUEUED",
                    format!(
                        "process_id={} dispatched={dispatched} items={batch_size}",
                        job.process_id
                    ),
                );
            }
            super::retry::JobOutcome::Failed(message) => {
                let undispatched = targets.len().saturating_sub(dispatched);
                repository
                    .fail_remaining_items(&job.process_id, undispatched as u64, &message)
                    .await?;
                return Ok(BulkDispatchOutcome::Failed {
                    dispatched,
                    undispatched,
                    message,
                });
            }
        }
    }
    Ok(BulkDispatchOutcome::Complete { dispatched })
}

fn file_processing_job(
    operation: &BulkOperation,
    process_id: &str,
    target: crate::models::bulk_operation_model::BulkFileTarget,
) -> FileProcessingJob {
    match operation {
        BulkOperation::Delete => FileProcessingJob::UpdateMetadata(UpdateFileMetadataJob {
            process_id: process_id.to_owned(),
            drive_id: target.drive_id,
            path: target.path,
            favorite: None,
            is_trashed: Some(true),
            add_tags: Vec::new(),
            add_collection_names: Vec::new(),
        }),
        BulkOperation::AddToFavorites => FileProcessingJob::UpdateMetadata(UpdateFileMetadataJob {
            process_id: process_id.to_owned(),
            drive_id: target.drive_id,
            path: target.path,
            favorite: Some(true),
            is_trashed: None,
            add_tags: Vec::new(),
            add_collection_names: Vec::new(),
        }),
        BulkOperation::AddToCollection { collection_name } => {
            FileProcessingJob::UpdateMetadata(UpdateFileMetadataJob {
                process_id: process_id.to_owned(),
                drive_id: target.drive_id,
                path: target.path,
                favorite: None,
                is_trashed: None,
                add_tags: Vec::new(),
                add_collection_names: vec![collection_name.clone()],
            })
        }
        BulkOperation::AddTag { tag } => FileProcessingJob::UpdateMetadata(UpdateFileMetadataJob {
            process_id: process_id.to_owned(),
            drive_id: target.drive_id,
            path: target.path,
            favorite: None,
            is_trashed: None,
            add_tags: vec![tag.clone()],
            add_collection_names: Vec::new(),
        }),
        BulkOperation::EmptyTrash => FileProcessingJob::Delete(DeleteFileJob {
            process_id: process_id.to_owned(),
            drive_id: target.drive_id,
            file_id: target.file_id,
            path: target.path,
        }),
    }
}

async fn collect_targets_with_retry(
    job: &BulkOperationJob,
    bulk_operations: BulkOperationService,
) -> Result<Vec<crate::models::bulk_operation_model::BulkFileTarget>, String> {
    for attempt in 0..=3 {
        match bulk_operations
            .collect_targets(
                job.filters.clone(),
                job.snapshot_at_ms,
                job.selected_ids.clone(),
            )
            .await
        {
            Ok(targets) => return Ok(targets),
            Err(error) if attempt == 3 => return Err(format!("{error:?}")),
            Err(error) => {
                log_event(
                    BULK_OPERATION_QUEUE,
                    &format!("MATCH-RETRY {}/3", attempt + 1),
                    format!("process_id={} error={error:?}", job.process_id),
                );
                tokio::time::sleep(std::time::Duration::from_millis(100 * (attempt + 1))).await;
            }
        }
    }
    unreachable!("the retry loop returns after success or the final attempt")
}

#[cfg(test)]
#[path = "../../tests/workers/bulk_operation_worker.rs"]
mod tests;
