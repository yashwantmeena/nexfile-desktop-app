use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
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
        let backend = SqliteStorage::<ImportFileJob, (), ()>::new_in_queue(
            database.pool(),
            IMPORT_FILE_QUEUE,
        );
        let worker = WorkerBuilder::new(IMPORT_FILE_WORKER)
            .backend(backend)
            .build(move |job: ImportFileJob| {
                let service = service.clone();
                async move { consume_import_file(job, service).await }
            });
        let (shutdown, mut shutdown_receiver) = channel(1);
        let task = tauri::async_runtime::spawn(async move {
            worker
                .run_until(async move {
                    let _ = shutdown_receiver.recv().await;
                    Ok::<(), std::io::Error>(())
                })
                .await
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
