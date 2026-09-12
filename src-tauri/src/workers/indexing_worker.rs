use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use futures::FutureExt;
use tauri::async_runtime::{channel, JoinHandle, Mutex, Sender};

use crate::error::AppResult;
use crate::models::indexing_model::IndexingJob;
use crate::repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
use crate::repositories::database_repository::SqliteDatabase;
use crate::services::indexing_service::IndexingService;
use crate::utils::constants::{INDEXING_QUEUE, INDEXING_WORKER};
use crate::utils::operation_logger::log_event;

pub struct IndexingWorker {
    shutdown: Sender<()>,
    task: Mutex<Option<JoinHandle<Result<(), WorkerError>>>>,
}

impl IndexingWorker {
    pub fn start(database: &SqliteDatabase, service: IndexingService) -> Self {
        let queue_pool = database.pool().clone();
        let repository = SqliteBackgroundProcessingRepository::new(database.clone());
        let (shutdown, mut shutdown_receiver) = channel(1);
        let shutdown_signal = async move {
            let _ = shutdown_receiver.recv().await;
        }
        .boxed()
        .shared();
        let task = tauri::async_runtime::spawn(async move {
            log_event(INDEXING_WORKER, "START", "worker supervisor started");
            loop {
                let backend = SqliteStorage::<IndexingJob, (), ()>::new_with_config(
                    &queue_pool,
                    &super::queue_config(INDEXING_QUEUE),
                );
                let handler_service = service.clone();
                let handler_repository = repository.clone();
                let worker =
                    WorkerBuilder::new(format!("{}-{}", INDEXING_WORKER, uuid::Uuid::new_v4()))
                        .backend(backend)
                        .concurrency(1)
                        .build(move |job: IndexingJob| {
                            let service = handler_service.clone();
                            let repository = handler_repository.clone();
                            async move {
                                let subject = job.subject();
                                log_indexing_event("START", &job, "batch processing started");
                                repository.mark_running(job.process_id()).await?;
                                let outcome =
                                    super::retry::retry_and_ack(INDEXING_QUEUE, &subject, || {
                                        consume_indexing_job(job.clone(), service.clone())
                                    })
                                    .await?;
                                let failure = match &outcome {
                                    super::retry::JobOutcome::Completed => {
                                        log_indexing_event(
                                            "COMPLETE",
                                            &job,
                                            "batch processing completed",
                                        );
                                        None
                                    }
                                    super::retry::JobOutcome::Failed(message) => {
                                        log_indexing_event("FAILED", &job, message);
                                        Some(message.as_str())
                                    }
                                };
                                repository.finish_item(job.process_id(), failure).await?;
                                log_event(
                                    INDEXING_WORKER,
                                    "PERSISTED",
                                    format!(
                                        "process_id={} status={}",
                                        job.process_id(),
                                        if failure.is_some() {
                                            "failed"
                                        } else {
                                            "complete"
                                        }
                                    ),
                                );
                                Ok::<(), crate::error::AppError>(())
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
                    log_event(INDEXING_WORKER, "STOP", "worker supervisor stopped");
                    return result;
                }
                eprintln!("[{}][RESTART] worker exited: {result:?}", INDEXING_WORKER);
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
        log_event(INDEXING_WORKER, "STOP", "shutdown requested");
        let _ = self.shutdown.send(()).await;
        if let Some(task) = self.task.lock().await.take() {
            let _ = task.await;
        }
    }
}

async fn consume_indexing_job(job: IndexingJob, service: IndexingService) -> AppResult<()> {
    match job {
        IndexingJob::CreateIndexBatch { paths, .. } => {
            let existing = paths
                .into_iter()
                .filter(|path| path.is_file())
                .collect::<Vec<_>>();
            if existing.is_empty() {
                return Ok(());
            }
            service.process_batch(existing).await
        }
        IndexingJob::DeleteIndexBatch {
            drive_id, file_ids, ..
        } => service.delete_batch(drive_id, file_ids).await,
    }
}

fn log_indexing_event(event: &str, job: &IndexingJob, message: impl std::fmt::Display) {
    match job {
        IndexingJob::CreateIndexBatch { process_id, paths } => log_event(
            INDEXING_WORKER,
            event,
            format!(
                "process_id={process_id} operation=index items={} cleanup=identity | {message}",
                paths.len()
            ),
        ),
        IndexingJob::DeleteIndexBatch {
            process_id,
            drive_id,
            file_ids,
        } => log_event(
            INDEXING_WORKER,
            event,
            format!(
                "process_id={process_id} operation=delete-index drive_id={drive_id} items={} | {message}",
                file_ids.len()
            ),
        ),
    }
}
