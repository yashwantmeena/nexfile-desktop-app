use std::path::PathBuf;

use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use futures::FutureExt;
use sqlx::SqlitePool;
use tauri::async_runtime::{channel, JoinHandle, Mutex, Sender};

use crate::error::{AppError, AppResult};
use crate::models::image_processing_model::ImageProcessingJob;
use crate::models::indexing_model::IndexingJob;
use crate::repositories::database_repository::SqliteDatabase;
use crate::services::image_processing_service::ImageProcessingService;
use crate::utils::constants::{IMAGE_PROCESSING_QUEUE, IMAGE_PROCESSING_WORKER, INDEXING_QUEUE};

pub struct ImageProcessingWorker {
    shutdown: Sender<()>,
    task: Mutex<Option<JoinHandle<Result<(), WorkerError>>>>,
}

impl ImageProcessingWorker {
    pub fn start(
        database: &SqliteDatabase,
        clip_model_directory: PathBuf,
        florence2_model_directory: PathBuf,
        configs_directory: PathBuf,
    ) -> Self {
        let service = ImageProcessingService::new(
            clip_model_directory,
            florence2_model_directory,
            configs_directory,
        );
        let queue_pool = database.pool().clone();
        let (shutdown, mut shutdown_receiver) = channel(1);
        let shutdown_signal = async move {
            let _ = shutdown_receiver.recv().await;
        }
        .boxed()
        .shared();
        let task = tauri::async_runtime::spawn(async move {
            loop {
                let backend = SqliteStorage::<ImageProcessingJob, (), ()>::new_in_queue(
                    &queue_pool,
                    IMAGE_PROCESSING_QUEUE,
                );
                let handler_service = service.clone();
                let handler_pool = queue_pool.clone();
                let worker = WorkerBuilder::new(format!(
                    "{}-{}",
                    IMAGE_PROCESSING_WORKER,
                    uuid::Uuid::new_v4()
                ))
                .backend(backend)
                .concurrency(1)
                .build(move |job: ImageProcessingJob| {
                    let service = handler_service.clone();
                    let queue_pool = handler_pool.clone();
                    async move {
                        let subject = job.path.display().to_string();
                        super::retry::retry_and_ack(IMAGE_PROCESSING_QUEUE, &subject, || {
                            consume_image_processing_job(
                                job.clone(),
                                service.clone(),
                                queue_pool.clone(),
                            )
                        })
                        .await
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
                    return result;
                }
                eprintln!(
                    "[{}][RESTART] worker exited: {result:?}",
                    IMAGE_PROCESSING_WORKER
                );
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
        let _ = self.shutdown.send(()).await;
        if let Some(task) = self.task.lock().await.take() {
            let _ = task.await;
        }
    }
}

pub(crate) async fn consume_image_processing_job(
    job: ImageProcessingJob,
    service: ImageProcessingService,
    queue_pool: SqlitePool,
) -> AppResult<()> {
    let path = job.path.clone();
    if !path.is_file() {
        eprintln!(
            "[image-processing-queue][ACK] {} | stale source no longer exists",
            path.display()
        );
        return Ok(());
    }

    match service.process(job).await {
        Ok(output_path) => {
            publish_for_indexing(&queue_pool, &output_path).await?;
            eprintln!(
                "[image-processing-queue][ACK] {} | output={}",
                path.display(),
                output_path.display()
            );
            Ok(())
        }
        Err(_) if !path.is_file() => {
            eprintln!(
                "[image-processing-queue][ACK] {} | source disappeared during processing",
                path.display()
            );
            Ok(())
        }
        Err(error) => {
            eprintln!(
                "[image-processing-queue][RETRY] {} | {error:?}",
                path.display()
            );
            Err(error)
        }
    }
}

async fn publish_for_indexing(queue_pool: &SqlitePool, path: &PathBuf) -> AppResult<()> {
    let mut queue = SqliteStorage::<IndexingJob, (), ()>::new_in_queue(queue_pool, INDEXING_QUEUE);
    queue
        .push(IndexingJob { path: path.clone() })
        .await
        .map_err(AppError::database)
}
