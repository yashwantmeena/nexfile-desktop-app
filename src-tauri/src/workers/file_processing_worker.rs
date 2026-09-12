use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use futures::FutureExt;
use tauri::async_runtime::{channel, JoinHandle, Mutex, Sender};

use crate::error::{AppError, AppResult};
use crate::models::file_processing_model::{ExportFileJob, FileProcessingJob};
use crate::repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
use crate::repositories::database_repository::SqliteDatabase;
use crate::repositories::indexing_repository::TantivyIndexingRepository;
use crate::services::import_service::ImportService;
use crate::services::storage_service::StorageService;
use crate::utils::constants::{FILE_PROCESSING_QUEUE, FILE_PROCESSING_WORKER};
use crate::utils::operation_logger::log_event;

pub struct FileProcessingWorker {
    shutdown: Sender<()>,
    task: Mutex<Option<JoinHandle<Result<(), WorkerError>>>>,
}

impl FileProcessingWorker {
    pub fn start(
        database: &SqliteDatabase,
        imports: ImportService,
        storage: StorageService,
        index: TantivyIndexingRepository,
    ) -> Self {
        let queue_pool = database.pool().clone();
        let repository = SqliteBackgroundProcessingRepository::new(database.clone());
        let (shutdown, mut shutdown_receiver) = channel(1);
        let shutdown_signal = async move {
            let _ = shutdown_receiver.recv().await;
        }
        .boxed()
        .shared();
        let task = tauri::async_runtime::spawn(async move {
            log_event(FILE_PROCESSING_WORKER, "START", "worker supervisor started");
            loop {
                let backend = SqliteStorage::<FileProcessingJob, (), ()>::new_in_queue(
                    &queue_pool,
                    FILE_PROCESSING_QUEUE,
                );
                let handler_imports = imports.clone();
                let handler_storage = storage.clone();
                let handler_index = index.clone();
                let handler_repository = repository.clone();
                let worker = WorkerBuilder::new(format!(
                    "{}-{}",
                    FILE_PROCESSING_WORKER,
                    uuid::Uuid::new_v4()
                ))
                .backend(backend)
                .concurrency(1)
                .build(move |job: FileProcessingJob| {
                    let imports = handler_imports.clone();
                    let storage = handler_storage.clone();
                    let index = handler_index.clone();
                    let repository = handler_repository.clone();
                    async move {
                        let subject = job.subject();
                        log_event(
                            FILE_PROCESSING_WORKER,
                            "RECEIVED",
                            format!("process_id={} operation={} subject={subject}", job.process_id(), job.operation()),
                        );
                        repository.mark_running(job.process_id()).await?;
                        log_event(
                            FILE_PROCESSING_WORKER,
                            "RUNNING",
                            format!("process_id={} operation={} subject={subject}", job.process_id(), job.operation()),
                        );
                        let process_id = job.process_id().to_owned();
                        let operation = job.operation();
                        let outcome = super::retry::retry_and_ack(FILE_PROCESSING_QUEUE, &subject, || {
                            consume_file_processing_job(job.clone(), imports.clone(), storage.clone(), index.clone())
                        })
                        .await?;
                        let failure = match &outcome {
                            super::retry::JobOutcome::Completed => None,
                            super::retry::JobOutcome::Failed(message) => Some(message.as_str()),
                        };
                        repository.finish_item(&process_id, failure).await?;
                        log_event(
                            FILE_PROCESSING_WORKER,
                            if failure.is_some() { "FAILED" } else { "COMPLETE" },
                            format!("process_id={process_id} operation={operation} subject={subject}"),
                        );
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
                    log_event(FILE_PROCESSING_WORKER, "STOP", "worker supervisor stopped");
                    return result;
                }
                eprintln!("[{}][RESTART] worker exited: {result:?}", FILE_PROCESSING_WORKER);
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
        log_event(FILE_PROCESSING_WORKER, "STOP", "shutdown requested");
        let _ = self.shutdown.send(()).await;
        if let Some(task) = self.task.lock().await.take() {
            let _ = task.await;
        }
    }
}

async fn consume_file_processing_job(
    job: FileProcessingJob,
    imports: ImportService,
    storage: StorageService,
    index: TantivyIndexingRepository,
) -> AppResult<()> {
    match job {
        FileProcessingJob::Import(job) => imports.consume(job).await,
        FileProcessingJob::Export(job) => consume_export_file(job).await,
        FileProcessingJob::Delete(job) => {
            storage
                .permanently_delete_file(job.drive_id.clone(), job.file_id.clone(), job.path)
                .await?;
            tauri::async_runtime::spawn_blocking(move || index.delete_file(&job.drive_id, &job.file_id))
                .await
                .map_err(AppError::internal)??;
            Ok(())
        }
    }
}

async fn consume_export_file(job: ExportFileJob) -> AppResult<()> {
    let source = job.source.clone();
    let destination = job.destination.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(source, destination).map(|_| ()).map_err(AppError::from)
    })
    .await
    .map_err(AppError::internal)??;
    Ok(())
}
