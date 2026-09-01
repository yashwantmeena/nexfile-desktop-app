use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use tauri::async_runtime::{channel, JoinHandle, Mutex, Sender};

use crate::error::AppResult;
use crate::models::indexing_model::IndexingJob;
use crate::repositories::database_repository::SqliteDatabase;
use crate::services::indexing_service::IndexingService;
use crate::utils::constants::{INDEXING_QUEUE, INDEXING_WORKER};

pub struct IndexingWorker {
    shutdown: Sender<()>,
    task: Mutex<Option<JoinHandle<Result<(), WorkerError>>>>,
}

impl IndexingWorker {
    pub fn start(database: &SqliteDatabase, service: IndexingService) -> Self {
        let backend =
            SqliteStorage::<IndexingJob, (), ()>::new_in_queue(database.pool(), INDEXING_QUEUE);
        let worker = WorkerBuilder::new(INDEXING_WORKER)
            .backend(backend)
            .concurrency(1)
            .build(move |job: IndexingJob| {
                let service = service.clone();
                async move { consume_indexing_job(job, service).await }
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
                eprintln!("indexing worker stopped: {error:?}");
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

async fn consume_indexing_job(job: IndexingJob, service: IndexingService) -> AppResult<()> {
    let path = job.path.clone();
    if !path.is_file() {
        eprintln!(
            "[indexing-queue][ACK] {} | stale sidecar no longer exists",
            path.display()
        );
        return Ok(());
    }

    match service.process(job).await {
        Ok(()) => {
            eprintln!("[indexing-queue][ACK] {}", path.display());
            Ok(())
        }
        Err(_) if !path.is_file() => {
            eprintln!(
                "[indexing-queue][ACK] {} | sidecar disappeared during indexing",
                path.display()
            );
            Ok(())
        }
        Err(error) => {
            eprintln!("[indexing-queue][RETRY] {} | {error:?}", path.display());
            Err(error)
        }
    }
}
