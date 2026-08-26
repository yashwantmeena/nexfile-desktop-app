use std::path::{Path, PathBuf};

use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use futures::stream;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::background_process::{BackgroundProcess, BackgroundProcessStatus};
use crate::models::import::ImportFileJob;
use crate::repositories::background_processing::SqliteBackgroundProcessingRepository;
use crate::repositories::database::SqliteDatabase;
use crate::utils::constants::{
    APALIS_MIGRATION_TABLE, IMPORT_FILE_PROCESS_TYPE, IMPORT_FILE_QUEUE,
};

pub struct ImportService {
    repository: SqliteBackgroundProcessingRepository,
    queue_pool: SqlitePool,
}

impl ImportService {
    pub async fn new(
        repository: SqliteBackgroundProcessingRepository,
        database: &SqliteDatabase,
    ) -> AppResult<Self> {
        let queue_pool = database.pool().clone();
        let mut migrations = SqliteStorage::migrations();
        migrations.dangerous_set_table_name(APALIS_MIGRATION_TABLE);
        migrations
            .run(&queue_pool)
            .await
            .map_err(AppError::database)?;
        Ok(Self {
            repository,
            queue_pool,
        })
    }

    pub async fn import_files<I, P>(&self, paths: I) -> AppResult<BackgroundProcess>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let paths = validate_file_paths(paths)?;
        let total_items = u64::try_from(paths.len())
            .map_err(|_| AppError::validation("Too many files were selected."))?;
        let process = BackgroundProcess {
            process_id: uuid::Uuid::new_v4().to_string(),
            process_type: IMPORT_FILE_PROCESS_TYPE.to_owned(),
            status: BackgroundProcessStatus::Queued,
            priority: 0,
            total_items,
            processed_items: 0,
            failed_items: 0,
            remark: None,
            created_at_ms: 0,
            updated_at_ms: 0,
            started_at_ms: None,
            finished_at_ms: None,
        };
        let process = self.repository.insert(&process).await?;
        let process_id = process.process_id.clone();
        let mut jobs = stream::iter(paths.into_iter().map(|path| {
            Task::builder(ImportFileJob {
                process_id: process_id.clone(),
                path,
            })
            .build()
        }));
        let mut queue = SqliteStorage::<ImportFileJob, (), ()>::new_in_queue(
            &self.queue_pool,
            IMPORT_FILE_QUEUE,
        );

        if let Err(error) = queue.push_all(&mut jobs).await {
            let _ = self.repository.delete(&process.process_id).await;
            return Err(AppError::database(error));
        }

        Ok(process)
    }

    pub async fn close(&self) {
        self.repository.close().await;
    }
}

fn validate_file_paths<I, P>(paths: I) -> AppResult<Vec<PathBuf>>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let paths = paths
        .into_iter()
        .map(|path| path.as_ref().to_path_buf())
        .collect::<Vec<_>>();

    if paths.is_empty() {
        return Err(AppError::validation("At least one file is required."));
    }
    if paths.iter().any(|path| !path.is_file()) {
        return Err(AppError::validation(
            "Every selected path must point to an existing file.",
        ));
    }
    Ok(paths)
}
