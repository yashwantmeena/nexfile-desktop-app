use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use futures::stream;
use sqlx::SqlitePool;
use tauri::async_runtime::Mutex;

use crate::error::{AppError, AppResult, CounterOverflow};

use crate::models::background_process_model::BackgroundProcess;
use crate::models::file_model::ManagedFileMetadata;
use crate::models::file_processing_model::FileProcessingJob;
use crate::models::image_processing_model::ImageProcessingJob;
use crate::models::import_model::{ImportFileJob, ImportPreview};
use crate::models::storage_model::{DriveInfo, DriveMetadata};
use crate::repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
use crate::repositories::database_repository::SqliteDatabase;
use crate::repositories::storage_repository::SqliteStorageRepository;
use crate::services::image_processing_service::classification_output_path;
use crate::services::storage_service::{
    calculate_managed_statistics, drive_storage_root, read_drive_metadata, write_drive_metadata,
};
use crate::system::filesystem::get_drives;
use crate::utils::constants::{
    AI_PROCESSING_QUEUE, APALIS_MIGRATION_TABLE, FILE_PROCESSING_QUEUE,
    IMAGE_PROCESSING_PROCESS_TYPE, IMPORTED_FILES_DIRECTORY, IMPORT_FILE_ID_ALPHABET,
    IMPORT_FILE_ID_LENGTH, IMPORT_FILE_PROCESS_TYPE, IMPORT_FOLDER_PROCESS_TYPE,
};
use crate::utils::image_decoder::is_supported_image;
use crate::utils::operation_logger::log_event;

#[derive(Clone)]
pub struct ImportService {
    repository: Arc<SqliteBackgroundProcessingRepository>,
    storage_repository: Arc<SqliteStorageRepository>,
    queue_pool: SqlitePool,
    system_metadata_root: PathBuf,
    copy_lock: Arc<Mutex<()>>,
    metadata_lock: Arc<Mutex<()>>,
    search: Option<crate::repositories::indexing_repository::TantivyIndexingRepository>,
}

impl ImportService {
    pub(crate) fn collection_pool(&self) -> &SqlitePool {
        &self.queue_pool
    }

    pub(crate) fn background_processes(&self) -> &SqliteBackgroundProcessingRepository {
        &self.repository
    }

    pub async fn preview(&self, paths: Vec<String>, folder: bool) -> AppResult<ImportPreview> {
        log_event(
            FILE_PROCESSING_QUEUE,
            "PREVIEW",
            format!("requested_paths={} folder={folder}", paths.len()),
        );
        let paths = if folder {
            let path = paths
                .first()
                .ok_or_else(|| AppError::validation("Select a folder."))?
                .clone();
            tauri::async_runtime::spawn_blocking(move || collect_folder_files(Path::new(&path)))
                .await
                .map_err(AppError::internal)??
        } else {
            validate_file_paths(paths)?
        };
        let file_sizes = tauri::async_runtime::spawn_blocking(move || {
            paths
                .iter()
                .map(|path| std::fs::metadata(path).map(|metadata| metadata.len()))
                .collect::<Result<Vec<_>, _>>()
        })
        .await
        .map_err(AppError::internal)??;
        let file_count = u64::try_from(file_sizes.len())
            .map_err(|_| AppError::validation("Too many files were selected."))?;
        let total_bytes = file_sizes
            .iter()
            .try_fold(0_u64, |total, size| {
                total.checked_add(*size).ok_or(CounterOverflow)
            })
            .map_err(AppError::internal)?;

        let saved_drives = self.storage_repository.list().await?;
        let system_metadata_root = self.system_metadata_root.clone();
        let drives = tauri::async_runtime::spawn_blocking(get_drives)
            .await
            .map_err(AppError::internal)?;
        let mut capacities = drives
            .into_iter()
            .filter_map(|drive| {
                let on_drive = read_drive_metadata(&drive, &system_metadata_root)?;
                let saved = saved_drives
                    .iter()
                    .find(|saved| saved.drive_id == on_drive.drive_id && saved.is_mounted)?;
                Some((saved.priority, available_capacity(&drive, saved)))
            })
            .collect::<Vec<_>>();
        capacities.sort_by_key(|(priority, _)| *priority);
        let mounted_drive_count = u64::try_from(capacities.len())
            .map_err(|_| AppError::validation("Too many mounted drives."))?;
        let available_bytes = capacities
            .iter()
            .try_fold(0_u64, |total, (_, capacity)| {
                total.checked_add(*capacity).ok_or(CounterOverflow)
            })
            .map_err(AppError::internal)?;
        let mut remaining = capacities
            .into_iter()
            .map(|(_, capacity)| capacity)
            .collect::<Vec<_>>();
        let can_import_all = file_sizes.into_iter().all(|size| {
            let Some(capacity) = remaining.iter_mut().find(|capacity| **capacity >= size) else {
                return false;
            };
            *capacity -= size;
            true
        });

        let preview = ImportPreview {
            file_count,
            total_bytes,
            available_bytes,
            mounted_drive_count,
            can_import_all,
        };
        log_event(
            FILE_PROCESSING_QUEUE,
            "PREVIEW-COMPLETE",
            format!(
                "files={} bytes={} mounted_drives={} available_bytes={} can_import_all={}",
                preview.file_count,
                preview.total_bytes,
                preview.mounted_drive_count,
                preview.available_bytes,
                preview.can_import_all
            ),
        );
        Ok(preview)
    }

    pub async fn import_with_collections(
        &self,
        paths: Vec<String>,
        folder: bool,
        ids: Vec<String>,
    ) -> AppResult<BackgroundProcess> {
        log_event(
            FILE_PROCESSING_QUEUE,
            "REQUEST",
            format!(
                "requested_paths={} folder={} collections={}",
                paths.len(),
                folder,
                ids.len()
            ),
        );
        let names =
            crate::repositories::collection_repository::selected_names(&self.queue_pool, &ids)
                .await?;
        let paths = if folder {
            let path = paths
                .first()
                .ok_or_else(|| AppError::validation("Select a folder."))?
                .clone();
            tauri::async_runtime::spawn_blocking(move || collect_folder_files(Path::new(&path)))
                .await
                .map_err(AppError::internal)??
        } else {
            validate_file_paths(paths)?
        };
        let process = self
            .queue_files_with_collections(
                paths,
                if folder {
                    IMPORT_FOLDER_PROCESS_TYPE
                } else {
                    IMPORT_FILE_PROCESS_TYPE
                },
                names,
            )
            .await?;
        log_event(
            FILE_PROCESSING_QUEUE,
            "REQUEST-COMPLETE",
            format!(
                "process_id={} total_items={}",
                process.process_id, process.total_items
            ),
        );
        Ok(process)
    }

    async fn save_selected_collections(
        &self,
        job: &ImportFileJob,
        root: &Path,
        drive_id: &str,
        destination: &Path,
    ) -> AppResult<()> {
        let encoded: Option<String> =
            sqlx::query_scalar("SELECT collections FROM background_processes WHERE process_id = ?")
                .bind(&job.process_id)
                .fetch_optional(&self.queue_pool)
                .await
                .map_err(AppError::database)?;
        let mut names: Vec<String> = match encoded {
            Some(encoded) => serde_json::from_str(&encoded).map_err(AppError::serialization)?,
            None => Vec::new(),
        };
        // Inherit only from the known source file, without scanning source or destination drives.
        if let Some(files) = job.path.parent().filter(|path| {
            path.file_name().and_then(|name| name.to_str()) == Some(IMPORTED_FILES_DIRECTORY)
        }) {
            if let Some(source_root) = files.parent() {
                if let Some(drive) = super::storage_service::read_metadata(
                    &source_root.join(crate::utils::constants::DRIVE_METADATA_FILE),
                ) {
                    let ids = super::file_service::sidecar_collection_ids(
                        &classification_output_path(&job.path),
                    )?;
                    if !ids.is_empty() {
                        let metadata =
                            super::collection_service::read(source_root, &drive.drive_id)?;
                        for id in ids {
                            let item = metadata
                                .collections
                                .iter()
                                .find(|item| item.id == id)
                                .ok_or_else(|| {
                                    AppError::validation(
                                        "Source file references an unknown collection.",
                                    )
                                })?;
                            names.push(item.name.clone());
                        }
                    }
                }
            }
        }
        super::collection_service::add_file_to_collections(root, drive_id, destination, &names)
    }
    pub fn with_search_index(
        mut self,
        search: crate::repositories::indexing_repository::TantivyIndexingRepository,
    ) -> Self {
        self.search = Some(search);
        self
    }

    async fn index_filename(
        &self,
        drive: &str,
        job: &ImportFileJob,
        destination: &Path,
    ) -> AppResult<()> {
        if let Some(search) = &self.search {
            let search = search.clone();
            let drive = drive.to_owned();
            let file = job.file_id.clone();
            let name = job
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let sidecar = crate::services::file_service::read_managed_file_metadata(
                &classification_output_path(destination),
            )?;
            let collection_ids = sidecar.collection_ids;
            let favorite = sidecar.favorite;
            log_event(
                FILE_PROCESSING_QUEUE,
                "INDEX-START",
                format!("drive_id={drive} file_id={file}"),
            );
            tauri::async_runtime::spawn_blocking(move || {
                search.index_filename(&drive, &file, &name, &collection_ids, favorite)
            })
            .await
            .map_err(AppError::internal)??;
            log_event(FILE_PROCESSING_QUEUE, "INDEXED", "file metadata indexed");
        }
        Ok(())
    }

    pub(crate) fn metadata_lock(&self) -> &Mutex<()> {
        &self.metadata_lock
    }

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
            metadata_lock: Arc::new(Mutex::new(())),
            search: None,
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
        self.queue_files_with_collections(paths, process_type, Vec::new())
            .await
    }

    async fn queue_files_with_collections(
        &self,
        paths: Vec<PathBuf>,
        process_type: &str,
        names: Vec<String>,
    ) -> AppResult<BackgroundProcess> {
        let total_items = u64::try_from(paths.len())
            .map_err(|_| AppError::validation("Too many files were selected."))?;
        let process = self
            .repository
            .acquire_import(process_type, total_items, &names)
            .await?;
        let process_id = process.process_id.clone();
        log_event(
            FILE_PROCESSING_QUEUE,
            "QUEUING",
            format!("process_id={process_id} process_type={process_type} items={total_items}"),
        );
        let mut jobs = stream::iter(paths.into_iter().map(|path| {
            Task::builder(FileProcessingJob::Import(ImportFileJob {
                process_id: process_id.clone(),
                file_id: nanoid::nanoid!(IMPORT_FILE_ID_LENGTH, &IMPORT_FILE_ID_ALPHABET),
                path,
            }))
            .build()
        }));
        let mut queue = SqliteStorage::<FileProcessingJob, (), ()>::new_in_queue(
            &self.queue_pool,
            FILE_PROCESSING_QUEUE,
        );
        if let Err(error) = queue.push_all(&mut jobs).await {
            let _ = self
                .repository
                .rollback_items(&process.process_id, total_items)
                .await;
            return Err(AppError::database(error));
        }
        log_event(
            FILE_PROCESSING_QUEUE,
            "QUEUED",
            format!(
                "process_id={} process_type={process_type} items={total_items}",
                process.process_id
            ),
        );
        Ok(process)
    }
    pub async fn consume(&self, job: ImportFileJob) -> AppResult<()> {
        log_event(
            FILE_PROCESSING_QUEUE,
            "CONSUME",
            format!(
                "process_id={} file_id={} source={}",
                job.process_id,
                job.file_id,
                job.path.display()
            ),
        );
        let _guard = self.copy_lock.lock().await;
        let drives = tauri::async_runtime::spawn_blocking(get_drives)
            .await
            .map_err(AppError::internal)?;
        self.consume_with_drives(job, drives).await
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
        log_event(
            FILE_PROCESSING_QUEUE,
            "CANDIDATES",
            format!(
                "process_id={} file_id={} candidates={} size_bytes={file_size}",
                job.process_id,
                job.file_id,
                candidates.len()
            ),
        );

        for (drive, mut saved) in candidates {
            log_event(
                FILE_PROCESSING_QUEUE,
                "TRY-DRIVE",
                format!(
                    "process_id={} file_id={} drive_id={} priority={}",
                    job.process_id, job.file_id, saved.drive_id, saved.priority
                ),
            );
            let files_directory = drive_storage_root(&drive, &self.system_metadata_root)
                .join(IMPORTED_FILES_DIRECTORY);
            let destination = files_directory.join(destination_name(&job));

            if destination.try_exists()? {
                log_event(
                    FILE_PROCESSING_QUEUE,
                    "DESTINATION-EXISTS",
                    format!(
                        "process_id={} file_id={} drive_id={} path={}",
                        job.process_id,
                        job.file_id,
                        saved.drive_id,
                        destination.display()
                    ),
                );
                let _metadata_guard = self.metadata_lock.lock().await;
                if std::fs::metadata(&destination)?.len() != source_metadata.len() {
                    return Err(AppError::internal(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "the import destination already exists with a different size",
                    )));
                }
                write_import_sidecar(&job, &destination)?;
                self.save_selected_collections(
                    &job,
                    &drive_storage_root(&drive, &self.system_metadata_root),
                    &saved.drive_id,
                    &destination,
                )
                .await?;
                let (file_count, app_used_bytes) = calculate_managed_statistics(&files_directory)?;
                saved.file_count = file_count;
                saved.app_used_bytes = app_used_bytes;

                self.storage_repository
                    .update(&saved, |updated| {
                        write_drive_metadata(&drive, &self.system_metadata_root, updated)
                    })
                    .await?;
                drop(_metadata_guard);
                self.index_filename(&saved.drive_id, &job, &destination)
                    .await?;
                self.publish_for_image_processing(&destination).await?;
                log_event(
                    FILE_PROCESSING_QUEUE,
                    "COMPLETE",
                    format!(
                        "process_id={} file_id={} drive_id={} reused_destination=true",
                        job.process_id, job.file_id, saved.drive_id
                    ),
                );
                return Ok(());
            }

            if !can_fit(&drive, &saved, file_size) {
                log_event(
                    FILE_PROCESSING_QUEUE,
                    "SKIP-DRIVE",
                    format!(
                        "process_id={} file_id={} drive_id={} reason=insufficient_capacity",
                        job.process_id, job.file_id, saved.drive_id
                    ),
                );
                continue;
            }

            std::fs::create_dir_all(&files_directory)?;
            let temporary = files_directory.join(format!(".{}.importing", job.file_id));
            // Keep imports serialized, but let browsing proceed while bytes are copied.
            let source = job.path.clone();
            let staging_path = temporary.clone();
            let expected_size = source_metadata.len();
            log_event(
                FILE_PROCESSING_QUEUE,
                "COPY-START",
                format!(
                    "process_id={} file_id={} drive_id={} staging={}",
                    job.process_id,
                    job.file_id,
                    saved.drive_id,
                    temporary.display()
                ),
            );
            let copied = tauri::async_runtime::spawn_blocking(move || {
                copy_to_temporary(&source, &staging_path, expected_size)
            })
            .await
            .map_err(AppError::internal)?;
            match copied {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::StorageFull => {
                    let _ = std::fs::remove_file(&temporary);
                    continue;
                }
                Err(error) => return Err(error.into()),
            }
            log_event(
                FILE_PROCESSING_QUEUE,
                "COPY-COMPLETE",
                format!(
                    "process_id={} file_id={} drive_id={} bytes={file_size}",
                    job.process_id, job.file_id, saved.drive_id
                ),
            );

            // Publish only a complete file. Readers share this short publication lock,
            // never the lock that serializes the potentially slow copy.
            let _metadata_guard = self.metadata_lock.lock().await;
            std::fs::rename(&temporary, &destination)?;
            write_import_sidecar(&job, &destination)?;
            self.save_selected_collections(
                &job,
                &drive_storage_root(&drive, &self.system_metadata_root),
                &saved.drive_id,
                &destination,
            )
            .await?;
            saved.file_count = saved
                .file_count
                .checked_add(1)
                .ok_or_else(|| AppError::internal(CounterOverflow))?;
            saved.app_used_bytes = saved
                .app_used_bytes
                .checked_add(file_size)
                .ok_or_else(|| AppError::internal(CounterOverflow))?;

            self.storage_repository
                .update(&saved, |updated| {
                    write_drive_metadata(&drive, &self.system_metadata_root, updated)
                })
                .await?;
            drop(_metadata_guard);
            self.index_filename(&saved.drive_id, &job, &destination)
                .await?;
            self.publish_for_image_processing(&destination).await?;
            log_event(
                FILE_PROCESSING_QUEUE,
                "COMPLETE",
                format!(
                    "process_id={} file_id={} drive_id={} reused_destination=false",
                    job.process_id, job.file_id, saved.drive_id
                ),
            );
            return Ok(());
        }

        Err(AppError::storage_unavailable(
            "No mounted drive has enough available space for this file.",
        ))
    }

    async fn publish_for_image_processing(&self, path: &Path) -> AppResult<()> {
        if !is_supported_image(path) {
            log_event(
                AI_PROCESSING_QUEUE,
                "SKIP",
                format!("path={} reason=unsupported_image", path.display()),
            );
            return Ok(());
        }
        let mut queue = SqliteStorage::<ImageProcessingJob, (), ()>::new_in_queue(
            &self.queue_pool,
            AI_PROCESSING_QUEUE,
        );
        let process = self
            .repository
            .acquire_stage(IMAGE_PROCESSING_PROCESS_TYPE, 1)
            .await?;
        if let Err(error) = queue
            .push(ImageProcessingJob {
                process_id: process.process_id.clone(),
                path: path.to_path_buf(),
            })
            .await
        {
            let _ = self.repository.rollback_items(&process.process_id, 1).await;
            return Err(AppError::database(error));
        }
        log_event(
            AI_PROCESSING_QUEUE,
            "QUEUED",
            format!("process_id={} path={}", process.process_id, path.display()),
        );
        Ok(())
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
    u64::try_from(file_size).is_ok_and(|file_size| available_capacity(drive, metadata) >= file_size)
}

fn available_capacity(drive: &DriveInfo, metadata: &DriveMetadata) -> u64 {
    let physical_available = drive
        .total_bytes
        .saturating_sub(drive.system_used_bytes)
        .max(0);
    let app_available = metadata
        .app_limit_bytes
        .map(|limit| limit.saturating_sub(metadata.app_used_bytes).max(0))
        .unwrap_or(physical_available);
    u64::try_from(physical_available.min(app_available)).unwrap_or(0)
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

fn write_import_sidecar(job: &ImportFileJob, destination: &Path) -> AppResult<()> {
    let name = job
        .path
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| AppError::validation("The imported file has no filename."))?
        .to_string_lossy()
        .into_owned();
    let path = classification_output_path(destination);
    if path.try_exists()? {
        return Ok(());
    }
    let created_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(AppError::system_time)?
        .as_millis();
    let created_at_ms = u64::try_from(created_at_ms).map_err(AppError::internal)?;
    let temporary = path.with_file_name(format!(".{}.sidecar.tmp", job.file_id));
    let bytes = serde_json::to_vec(&ManagedFileMetadata {
        version: 1,
        name,
        created_at_ms,
        favorite: false,
        is_trashed: false,
        trashed_at_ms: None,
        is_deleted: false,
        collection_ids: Vec::new(),
        search_keywords: Vec::new(),
        captured_at_ms: None,
    })
    .map_err(AppError::serialization)?;
    std::fs::write(&temporary, bytes)?;
    if let Err(error) = std::fs::rename(&temporary, &path) {
        if path.try_exists()? {
            let _ = std::fs::remove_file(&temporary);
            return Ok(());
        }
        let _ = std::fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
}

fn copy_to_temporary(source: &Path, temporary: &Path, expected_size: u64) -> std::io::Result<()> {
    let copied = std::fs::copy(source, temporary)?;
    if copied != expected_size {
        let _ = std::fs::remove_file(temporary);
        return Err(std::io::Error::new(
            std::io::ErrorKind::WriteZero,
            "the complete source file was not copied",
        ));
    }
    Ok(())
}

#[cfg(all(test, target_os = "windows"))]
#[path = "../../tests/services/import_service_concurrency.rs"]
mod concurrency_tests;
