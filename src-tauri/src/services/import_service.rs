use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use futures::stream;
use sqlx::SqlitePool;
use tauri::async_runtime::Mutex;

use crate::error::{AppError, AppResult};
use crate::models::background_process_model::{BackgroundProcess, BackgroundProcessStatus};
use crate::models::import_model::ImportFileJob;
use crate::models::storage_model::{DriveInfo, DriveMetadata};
use crate::repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
use crate::repositories::database_repository::SqliteDatabase;
use crate::repositories::storage_repository::SqliteStorageRepository;
use crate::services::storage_service::{
    drive_storage_root, read_drive_metadata, write_drive_metadata,
};
use crate::system::filesystem::get_drives;
use crate::utils::constants::{
    APALIS_MIGRATION_TABLE, IMPORTED_FILES_DIRECTORY, IMPORT_FILE_ID_ALPHABET,
    IMPORT_FILE_ID_LENGTH, IMPORT_FILE_PROCESS_TYPE, IMPORT_FILE_QUEUE, IMPORT_FOLDER_PROCESS_TYPE,
};

#[derive(Clone)]
pub struct ImportService {
    repository: Arc<SqliteBackgroundProcessingRepository>,
    storage_repository: Arc<SqliteStorageRepository>,
    queue_pool: SqlitePool,
    system_metadata_root: PathBuf,
    copy_lock: Arc<Mutex<()>>,
}

impl ImportService {
    pub async fn new(
        repository: SqliteBackgroundProcessingRepository,
        storage_repository: SqliteStorageRepository,
        database: &SqliteDatabase,
        system_metadata_root: PathBuf,
    ) -> AppResult<Self> {
        let queue_pool = database.pool().clone();
        let mut migrations = SqliteStorage::migrations();
        migrations.dangerous_set_table_name(APALIS_MIGRATION_TABLE);
        migrations
            .run(&queue_pool)
            .await
            .map_err(AppError::database)?;
        Ok(Self {
            repository: Arc::new(repository),
            storage_repository: Arc::new(storage_repository),
            queue_pool,
            system_metadata_root,
            copy_lock: Arc::new(Mutex::new(())),
        })
    }

    pub async fn import_files<I, P>(&self, paths: I) -> AppResult<BackgroundProcess>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let paths = validate_file_paths(paths)?;
        self.queue_files(paths, IMPORT_FILE_PROCESS_TYPE).await
    }

    pub async fn import_folder(&self, path: impl AsRef<Path>) -> AppResult<BackgroundProcess> {
        let path = path.as_ref().to_path_buf();
        let paths = tauri::async_runtime::spawn_blocking(move || collect_folder_files(&path))
            .await
            .map_err(AppError::internal)??;
        self.queue_files(paths, IMPORT_FOLDER_PROCESS_TYPE).await
    }

    async fn queue_files(
        &self,
        paths: Vec<PathBuf>,
        process_type: &str,
    ) -> AppResult<BackgroundProcess> {
        let total_items = u64::try_from(paths.len())
            .map_err(|_| AppError::validation("Too many files were selected."))?;
        let process = BackgroundProcess {
            process_id: uuid::Uuid::new_v4().to_string(),
            process_type: process_type.to_owned(),
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
                file_id: nanoid::nanoid!(IMPORT_FILE_ID_LENGTH, &IMPORT_FILE_ID_ALPHABET),
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

    pub async fn consume(&self, job: ImportFileJob) -> AppResult<()> {
        let _guard = self.copy_lock.lock().await;
        self.consume_with_drives(job, get_drives()).await
    }

    async fn consume_with_drives(
        &self,
        job: ImportFileJob,
        connected_drives: Vec<DriveInfo>,
    ) -> AppResult<()> {
        validate_job(&job)?;
        let source_metadata = std::fs::metadata(&job.path)?;
        if !source_metadata.is_file() {
            return Err(AppError::validation(
                "The queued import path is no longer a file.",
            ));
        }
        let file_size = i64::try_from(source_metadata.len()).map_err(AppError::internal)?;
        let saved_drives = self.storage_repository.list().await?;
        let mut candidates = connected_drives
            .into_iter()
            .filter_map(|drive| {
                let on_drive = read_drive_metadata(&drive, &self.system_metadata_root)?;
                let saved = saved_drives
                    .iter()
                    .find(|saved| saved.drive_id == on_drive.drive_id && saved.is_mounted)?;
                Some((drive, saved.clone()))
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(_, metadata)| metadata.priority);

        for (drive, mut saved) in candidates {
            if !can_fit(&drive, &saved, file_size) {
                continue;
            }

            let files_directory = drive_storage_root(&drive, &self.system_metadata_root)
                .join(IMPORTED_FILES_DIRECTORY);
            let destination = files_directory.join(destination_name(&job));

            if destination.try_exists()? {
                if std::fs::metadata(&destination)?.len() != source_metadata.len() {
                    return Err(AppError::internal(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "the import destination already exists with a different size",
                    )));
                }
                let (file_count, app_used_bytes) = calculate_usage(&files_directory)?;
                saved.file_count = file_count;
                saved.app_used_bytes = app_used_bytes;
                let updated = self.storage_repository.update(&saved).await?;
                write_drive_metadata(&drive, &self.system_metadata_root, &updated)?;
                return Ok(());
            }

            std::fs::create_dir_all(&files_directory)?;
            let temporary = files_directory.join(format!(".{}.importing", job.file_id));
            match copy_atomically(&job.path, &temporary, &destination, source_metadata.len()) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::StorageFull => {
                    let _ = std::fs::remove_file(&temporary);
                    continue;
                }
                Err(error) => return Err(error.into()),
            }

            saved.file_count = saved
                .file_count
                .checked_add(1)
                .ok_or_else(|| AppError::internal(CounterOverflow))?;
            saved.app_used_bytes = saved
                .app_used_bytes
                .checked_add(file_size)
                .ok_or_else(|| AppError::internal(CounterOverflow))?;
            let updated = self.storage_repository.update(&saved).await?;
            write_drive_metadata(&drive, &self.system_metadata_root, &updated)?;
            return Ok(());
        }

        Err(AppError::storage_unavailable(
            "No mounted drive has enough available space for this file.",
        ))
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

fn collect_folder_files(root: &Path) -> AppResult<Vec<PathBuf>> {
    if !root.is_dir() {
        return Err(AppError::validation(
            "The selected folder must be an existing directory.",
        ));
    }

    let mut directories = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = directories.pop() {
        let mut entries = std::fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(std::fs::DirEntry::path);

        for entry in entries {
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                directories.push(entry.path());
            } else if file_type.is_file() {
                files.push(entry.path());
            }
        }
    }

    if files.is_empty() {
        return Err(AppError::validation(
            "The selected folder does not contain any files.",
        ));
    }

    files.sort();
    Ok(files)
}

fn validate_job(job: &ImportFileJob) -> AppResult<()> {
    if job.file_id.len() != IMPORT_FILE_ID_LENGTH
        || !job.file_id.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err(AppError::validation(
            "The queued import file ID is invalid.",
        ));
    }
    Ok(())
}

fn can_fit(drive: &DriveInfo, metadata: &DriveMetadata, file_size: i64) -> bool {
    let physical_available = drive.total_bytes.saturating_sub(drive.system_used_bytes);
    let app_available = metadata
        .app_limit_bytes
        .map(|limit| limit.saturating_sub(metadata.app_used_bytes))
        .unwrap_or(physical_available);
    file_size <= physical_available && file_size <= app_available
}

fn destination_name(job: &ImportFileJob) -> OsString {
    let mut name = OsString::from(&job.file_id);
    if let Some(extension) = job
        .path
        .extension()
        .filter(|extension| !extension.is_empty())
    {
        name.push(".");
        name.push(extension);
    }
    name
}

fn copy_atomically(
    source: &Path,
    temporary: &Path,
    destination: &Path,
    expected_size: u64,
) -> std::io::Result<()> {
    let copied = std::fs::copy(source, temporary)?;
    if copied != expected_size {
        let _ = std::fs::remove_file(temporary);
        return Err(std::io::Error::new(
            std::io::ErrorKind::WriteZero,
            "the complete source file was not copied",
        ));
    }
    std::fs::rename(temporary, destination)
}

fn calculate_usage(files_directory: &Path) -> AppResult<(i64, i64)> {
    let mut file_count = 0_i64;
    let mut app_used_bytes = 0_i64;
    for entry in std::fs::read_dir(files_directory)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if !metadata.is_file() || entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        file_count = file_count
            .checked_add(1)
            .ok_or_else(|| AppError::internal(CounterOverflow))?;
        let size = i64::try_from(metadata.len()).map_err(AppError::internal)?;
        app_used_bytes = app_used_bytes
            .checked_add(size)
            .ok_or_else(|| AppError::internal(CounterOverflow))?;
    }
    Ok((file_count, app_used_bytes))
}

#[derive(Debug, thiserror::Error)]
#[error("the import metadata counter overflowed")]
struct CounterOverflow;
