use std::path::PathBuf;

use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use tauri::async_runtime::{channel, JoinHandle, Mutex, Sender};

use crate::models::image_processing_model::ImageProcessingJob;
use crate::repositories::database_repository::SqliteDatabase;
use crate::services::image_processing_service::ImageProcessingService;
use crate::utils::constants::{IMAGE_PROCESSING_QUEUE, IMAGE_PROCESSING_WORKER};

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
        let worker = WorkerBuilder::new(IMAGE_PROCESSING_WORKER)
            .backend(backend)
            .concurrency(1)
            .build(move |job: ImageProcessingJob| {
                let service = service.clone();
                async move { consume_image_processing_job(job, service).await }
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

async fn consume_image_processing_job(
    job: ImageProcessingJob,
    service: ImageProcessingService,
) -> crate::error::AppResult<()> {
    let path = job.path.clone();
    if !path.is_file() {
        eprintln!(
            "skipped stale image-processing job because the source file no longer exists: {}",
            path.display()
        );
        return Ok(());
    }

    match service.process(job).await {
        Ok(output_path) => {
            eprintln!(
                "captioned and classified image {} -> {}",
                path.display(),
                output_path.display()
            );
            Ok(())
        }
        Err(_) if !path.is_file() => {
            eprintln!(
                "skipped stale image-processing job because the source file disappeared: {}",
                path.display()
            );
            Ok(())
        }
        Err(error) => {
            eprintln!("image processing failed for {}: {error:?}", path.display());
            Err(error)
        }
    }
}

#[cfg(test)]
#[path = "../../tests/workers/image_processing_worker.rs"]
mod tests;
