use std::path::{Path, PathBuf};
use std::sync::Arc;

use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use futures::stream;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::delete_model::DeleteFileJob;
use crate::models::storage_model::{DriveInfo, DriveMetadata};
use crate::repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
use crate::repositories::database_repository::SqliteDatabase;
use crate::repositories::storage_repository::SqliteStorageRepository;
use crate::services::image_processing_service::classification_output_path;
use crate::services::storage_service::{drive_storage_root, read_drive_metadata};
use crate::system::filesystem::get_drives;
use crate::utils::constants::{DELETE_FILE_PROCESS_TYPE, DELETE_FILE_QUEUE, IMPORTED_FILES_DIRECTORY};
use crate::utils::operation_logger::log_event;

/// Coordinates the transition from soft-deleted files to durable permanent-delete jobs.
#[derive(Clone)]
pub struct TrashService {
    repository: Arc<SqliteBackgroundProcessingRepository>,
    storage_repository: Arc<SqliteStorageRepository>,
    queue_pool: SqlitePool,
    system_metadata_root: PathBuf,
}

impl TrashService {
    pub fn new(
        repository: SqliteBackgroundProcessingRepository,
        storage_repository: SqliteStorageRepository,
        database: &SqliteDatabase,
        system_metadata_root: PathBuf,
    ) -> Self {
        Self {
            repository: Arc::new(repository),
            storage_repository: Arc::new(storage_repository),
            queue_pool: database.pool().clone(),
            system_metadata_root,
        }
    }

    /// Publish every soft-deleted managed file to the durable deletion queue. The worker rechecks
    /// the deletion state before removing bytes, so a restored item remains safe.
    pub async fn empty_trash(&self) -> AppResult<()> {
        log_event(DELETE_FILE_QUEUE, "REQUEST", "empty Trash requested");
        let snapshots = self.storage_repository.list().await?;
        let root = self.system_metadata_root.clone();
        let jobs = tauri::async_runtime::spawn_blocking(move || {
            collect_deleted_file_jobs(snapshots, get_drives(), &root)
        })
        .await
        .map_err(AppError::internal)??;
        let total_items = u64::try_from(jobs.len())
            .map_err(|_| AppError::validation("Too many files are in Trash."))?;
        log_event(
            DELETE_FILE_QUEUE,
            "DISCOVERED",
            format!("trash_items={total_items}"),
        );
        if total_items == 0 {
            log_event(DELETE_FILE_QUEUE, "COMPLETE", "Trash is already empty");
            return Ok(());
        }

        let process = self
            .repository
            .acquire_stage(DELETE_FILE_PROCESS_TYPE, total_items)
            .await?;
        if let Err(error) = tauri::async_runtime::spawn_blocking({
            let jobs = jobs.clone();
            move || mark_jobs_as_deleted(&jobs)
        })
        .await
        .map_err(AppError::internal)?
        {
            let _ = restore_queued_job_state(&jobs);
            let _ = self
                .repository
                .rollback_items(&process.process_id, total_items)
                .await;
            return Err(error);
        }
        log_event(
            DELETE_FILE_QUEUE,
            "MARKED",
            format!("process_id={} items={total_items}", process.process_id),
        );

        let process_id = process.process_id.clone();
        let queued_jobs = jobs.clone();
        let mut jobs = stream::iter(jobs.into_iter().map(|mut job| {
            job.process_id = process_id.clone();
            Task::builder(job).build()
        }));
        let mut queue = SqliteStorage::<DeleteFileJob, (), ()>::new_in_queue(
            &self.queue_pool,
            DELETE_FILE_QUEUE,
        );
        if let Err(error) = queue.push_all(&mut jobs).await {
            let _ = restore_queued_job_state(&queued_jobs);
            let _ = self
                .repository
                .rollback_items(&process.process_id, total_items)
                .await;
            return Err(AppError::database(error));
        }
        log_event(
            DELETE_FILE_QUEUE,
            "QUEUED",
            format!("process_id={} items={total_items}", process.process_id),
        );
        Ok(())
    }
}

fn collect_deleted_file_jobs(
    snapshots: Vec<DriveMetadata>,
    connected: Vec<DriveInfo>,
    system_metadata_root: &Path,
) -> AppResult<Vec<DeleteFileJob>> {
    let mut jobs = Vec::new();
    for saved in snapshots {
        if !saved.is_mounted {
            continue;
        }
        let Some(drive) = connected
            .iter()
            .find(|drive| {
                read_drive_metadata(drive, system_metadata_root)
                    .is_some_and(|metadata| metadata.drive_id == saved.drive_id)
            })
            .cloned()
        else {
            continue;
        };
        let directory = drive_storage_root(&drive, system_metadata_root).join(IMPORTED_FILES_DIRECTORY);
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !entry.file_type().is_ok_and(|file_type| file_type.is_file())
                || entry.file_name().to_string_lossy().starts_with('.')
                || crate::services::storage_service::is_generated_image_sidecar(&path)
            {
                continue;
            }
            let sidecar = classification_output_path(&path);
            let Ok(metadata) = crate::services::file_service::read_managed_file_metadata(&sidecar) else {
                continue;
            };
            if !metadata.is_trashed || metadata.is_deleted {
                continue;
            }
            let Some(file_id) = path
                .file_stem()
                .and_then(|value| value.to_str())
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            jobs.push(DeleteFileJob {
                process_id: String::new(),
                drive_id: saved.drive_id.clone(),
                file_id: file_id.to_owned(),
                path,
            });
        }
    }
    Ok(jobs)
}

fn mark_jobs_as_deleted(jobs: &[DeleteFileJob]) -> AppResult<()> {
    for job in jobs {
        let sidecar = classification_output_path(&job.path);
        if !sidecar.try_exists()? {
            continue;
        }
        let metadata = crate::services::file_service::read_managed_file_metadata(&sidecar)?;
        if !metadata.is_trashed || metadata.is_deleted {
            continue;
        }
        crate::services::file_service::replace_sidecar_metadata(
            &sidecar, None, None, None, None, None, None, Some(true),
        )?;
    }
    Ok(())
}

fn restore_queued_job_state(jobs: &[DeleteFileJob]) -> AppResult<()> {
    for job in jobs {
        let sidecar = classification_output_path(&job.path);
        if !sidecar.try_exists()? {
            continue;
        }
        let metadata = crate::services::file_service::read_managed_file_metadata(&sidecar)?;
        if metadata.is_trashed && metadata.is_deleted {
            crate::services::file_service::replace_sidecar_metadata(
                &sidecar, None, None, None, None, None, None, Some(false),
            )?;
        }
    }
    Ok(())
}
