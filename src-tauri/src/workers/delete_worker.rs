use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use futures::FutureExt;
use tauri::async_runtime::{channel, JoinHandle, Mutex, Sender};

use crate::models::delete_model::DeleteFileJob;
use crate::repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
use crate::repositories::database_repository::SqliteDatabase;
use crate::repositories::indexing_repository::TantivyIndexingRepository;
use crate::services::storage_service::StorageService;
use crate::utils::constants::{DELETE_FILE_QUEUE, DELETE_FILE_WORKER};

pub struct DeleteWorker {
    shutdown: Sender<()>,
    task: Mutex<Option<JoinHandle<Result<(), WorkerError>>>>,
}

impl DeleteWorker {
    pub fn start(
        database: &SqliteDatabase,
        storage: StorageService,
        index: TantivyIndexingRepository,
    ) -> Self {
        let queue_pool = database.pool().clone();
        let repository = SqliteBackgroundProcessingRepository::new(database.clone());
        let (shutdown, mut shutdown_receiver) = channel(1);
        let shutdown_signal = async move { let _ = shutdown_receiver.recv().await; }.boxed().shared();
        let task = tauri::async_runtime::spawn(async move {
            loop {
                let backend = SqliteStorage::<DeleteFileJob, (), ()>::new_in_queue(
                    &queue_pool,
                    DELETE_FILE_QUEUE,
                );
                let handler_storage = storage.clone();
                let handler_index = index.clone();
                let handler_repository = repository.clone();
                let worker = WorkerBuilder::new(format!("{}-{}", DELETE_FILE_WORKER, uuid::Uuid::new_v4()))
                    .backend(backend)
                    .concurrency(1)
                    .build(move |job: DeleteFileJob| {
                        let storage = handler_storage.clone();
                        let index = handler_index.clone();
                        let repository = handler_repository.clone();
                        async move {
                            let subject = job.path.display().to_string();
                            repository.mark_running(&job.process_id).await?;
                            let outcome = super::retry::retry_and_ack(DELETE_FILE_QUEUE, &subject, || {
                                consume_delete_file(job.clone(), storage.clone(), index.clone())
                            }).await?;
                            let failure = match &outcome {
                                super::retry::JobOutcome::Completed => None,
                                super::retry::JobOutcome::Failed(message) => Some(message.as_str()),
                            };
                            repository.finish_item(&job.process_id, failure).await?;
                            Ok::<(), crate::error::AppError>(())
                        }
                    });
                let stop = shutdown_signal.clone();
                let result = worker.run_until(async move { stop.await; Ok::<(), std::io::Error>(()) }).await;
                if shutdown_signal.clone().now_or_never().is_some() {
                    return result;
                }
                eprintln!("[{}][RESTART] worker exited: {result:?}", DELETE_FILE_WORKER);
                tokio::select! {
                    _ = shutdown_signal.clone() => return Ok(()),
                    _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
                }
            }
        });
        Self { shutdown, task: Mutex::new(Some(task)) }
    }

    pub async fn close(&self) {
        let _ = self.shutdown.send(()).await;
        if let Some(task) = self.task.lock().await.take() { let _ = task.await; }
    }
}

async fn consume_delete_file(
    job: DeleteFileJob,
    storage: StorageService,
    index: TantivyIndexingRepository,
) -> crate::error::AppResult<()> {
    storage
        .permanently_delete_file(job.drive_id.clone(), job.file_id.clone(), job.path)
        .await?;
    tauri::async_runtime::spawn_blocking(move || index.delete_file(&job.drive_id, &job.file_id))
        .await
        .map_err(crate::error::AppError::internal)??;
    Ok(())
}
