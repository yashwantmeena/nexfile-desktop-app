use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use futures::FutureExt;
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
        let queue_pool = database.pool().clone();
        let (shutdown, mut shutdown_receiver) = channel(1);
        let shutdown_signal = async move {
            let _ = shutdown_receiver.recv().await;
        }
        .boxed()
        .shared();
        let task = tauri::async_runtime::spawn(async move {
            loop {
                let backend =
                    SqliteStorage::<IndexingJob, (), ()>::new_in_queue(&queue_pool, INDEXING_QUEUE);
                let handler_service = service.clone();
                let worker =
                    WorkerBuilder::new(format!("{}-{}", INDEXING_WORKER, uuid::Uuid::new_v4()))
                        .backend(backend)
                        .concurrency(1)
                        .build(move |job: IndexingJob| {
                            let service = handler_service.clone();
                            async move {
                                let subject = job.path.display().to_string();
                                super::retry::retry_and_ack(INDEXING_QUEUE, &subject, || {
                                    consume_indexing_job(job.clone(), service.clone())
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
