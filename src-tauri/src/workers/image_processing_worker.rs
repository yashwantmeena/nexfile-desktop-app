use std::path::PathBuf;

use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
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
        let backend = SqliteStorage::<ImageProcessingJob, (), ()>::new_in_queue(
            database.pool(),
            IMAGE_PROCESSING_QUEUE,
        );
        let queue_pool = database.pool().clone();
        let worker = WorkerBuilder::new(IMAGE_PROCESSING_WORKER)
            .backend(backend)
            .concurrency(1)
            .build(move |job: ImageProcessingJob| {
                let service = service.clone();
                let queue_pool = queue_pool.clone();
                async move { consume_image_processing_job(job, service, queue_pool).await }
            });
        let (shutdown, mut shutdown_receiver) = channel(1);
        let task = tauri::async_runtime::spawn(async move {
            let result = worker
                .run_until(async move {
                    let _ = shutdown_receiver.recv().await;
                    Ok::<(), std::io::Error>(())
                })
                .await;
            if let Err(error) = &result {
                eprintln!("image-processing worker stopped: {error:?}");
            }
            result
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
