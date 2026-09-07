use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use futures::FutureExt;
use tauri::async_runtime::{channel, JoinHandle, Mutex, Sender};

use crate::models::import_model::ImportFileJob;
use crate::repositories::database_repository::SqliteDatabase;
use crate::services::import_service::ImportService;
use crate::utils::constants::{IMPORT_FILE_QUEUE, IMPORT_FILE_WORKER};

pub struct ImportWorker {
    shutdown: Sender<()>,
    task: Mutex<Option<JoinHandle<Result<(), WorkerError>>>>,
}

impl ImportWorker {
    pub fn start(database: &SqliteDatabase, service: ImportService) -> Self {
        let queue_pool = database.pool().clone();
        let (shutdown, mut shutdown_receiver) = channel(1);
        let shutdown_signal = async move {
            let _ = shutdown_receiver.recv().await;
        }
        .boxed()
        .shared();
        let task = tauri::async_runtime::spawn(async move {
            loop {
                let backend = SqliteStorage::<ImportFileJob, (), ()>::new_in_queue(
                    &queue_pool,
                    IMPORT_FILE_QUEUE,
                );
                let handler_service = service.clone();
                let worker =
                    WorkerBuilder::new(format!("{}-{}", IMPORT_FILE_WORKER, uuid::Uuid::new_v4()))
                        .backend(backend)
                        .concurrency(1)
                        .build(move |job: ImportFileJob| {
                            let service = handler_service.clone();
                            async move {
                                let subject = job.path.display().to_string();
                                super::retry::retry_and_ack(IMPORT_FILE_QUEUE, &subject, || {
                                    consume_import_file(job.clone(), service.clone())
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
                    IMPORT_FILE_WORKER
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

async fn consume_import_file(
    job: ImportFileJob,
    service: ImportService,
) -> crate::error::AppResult<()> {
    service.consume(job).await
}
