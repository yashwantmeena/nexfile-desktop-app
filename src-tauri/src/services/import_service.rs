use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use futures::stream;
use sqlx::SqlitePool;
use tauri::async_runtime::Mutex;

use crate::error::{AppError, AppResult, CounterOverflow};
use crate::mappers::file_mapper::file_type_from_path;
use crate::models::background_process_model::BackgroundProcess;
use crate::models::file_model::ManagedFileMetadata;
use crate::models::image_processing_model::ImageProcessingJob;
use crate::models::import_model::ImportFileJob;
use crate::models::storage_model::{DriveInfo, DriveMetadata};
use crate::repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
use crate::repositories::database_repository::SqliteDatabase;
use crate::repositories::storage_repository::SqliteStorageRepository;
use crate::services::image_processing_service::classification_output_path;
use crate::services::storage_service::{
    calculate_managed_statistics, drive_storage_root, read_drive_metadata, write_drive_metadata,
};
use crate::system::filesystem::get_drives;
use crate::types::background_process_status::BackgroundProcessStatus;
use crate::utils::constants::{
    APALIS_MIGRATION_TABLE, IMAGE_PROCESSING_QUEUE, IMPORTED_FILES_DIRECTORY,
    IMPORT_FILE_ID_ALPHABET, IMPORT_FILE_ID_LENGTH, IMPORT_FILE_PROCESS_TYPE, IMPORT_FILE_QUEUE,
    IMPORT_FOLDER_PROCESS_TYPE,
};
use crate::utils::image_decoder::is_supported_image;

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
    pub(crate) fn collection_pool(&self) -> &SqlitePool { &self.queue_pool }

    pub async fn import_with_collections(&self, paths: Vec<String>, folder: bool, ids: Vec<String>) -> AppResult<BackgroundProcess> {
        let names = crate::repositories::collection_repository::selected_names(&self.queue_pool, &ids).await?;
        let paths = if folder {
            let path = paths.first().ok_or_else(|| AppError::validation("Select a folder."))?.clone();
            tauri::async_runtime::spawn_blocking(move || collect_folder_files(Path::new(&path))).await.map_err(AppError::internal)??
        } else { validate_file_paths(paths)? };
        self.queue_files_with_collections(paths, if folder { IMPORT_FOLDER_PROCESS_TYPE } else { IMPORT_FILE_PROCESS_TYPE }, names).await
    }

    async fn save_selected_collections(&self, job: &ImportFileJob, root: &Path, drive_id: &str, destination: &Path) -> AppResult<()> {
        let encoded: Option<String> = sqlx::query_scalar("SELECT collections FROM background_processes WHERE process_id = ?").bind(&job.process_id).fetch_optional(&self.queue_pool).await.map_err(AppError::database)?;
        let mut names: Vec<String> = match encoded { Some(encoded) => serde_json::from_str(&encoded).map_err(AppError::serialization)?, None => Vec::new() };
        // Inherit only from the known source file, without scanning source or destination drives.
        if let Some(files) = job.path.parent().filter(|path| path.file_name().and_then(|name| name.to_str()) == Some(IMPORTED_FILES_DIRECTORY)) {
            if let Some(source_root) = files.parent() {
                if let Some(drive) = super::storage_service::read_metadata(&source_root.join(crate::utils::constants::DRIVE_METADATA_FILE)) {
                    let ids = super::file_service::sidecar_collection_ids(&classification_output_path(&job.path))?;
                    if !ids.is_empty() {
                        let metadata = super::collection_service::read(source_root, &drive.drive_id)?;
                        for id in ids {
                            let item = metadata.collections.iter().find(|item| item.id == id).ok_or_else(|| AppError::validation("Source file references an unknown collection."))?;
                            names.push(item.name.clone());
                        }
                    }
                }
            }
        }
        assign_import_collections(root, drive_id, destination, &names)
    }
    pub fn with_search_index(
        mut self,
        search: crate::repositories::indexing_repository::TantivyIndexingRepository,
    ) -> Self {
        self.search = Some(search);
        self
    }

    async fn index_filename(&self, drive: &str, job: &ImportFileJob) -> AppResult<()> {
        if let Some(search) = &self.search {
            let search = search.clone();
            let drive = drive.to_owned();
            let file = job.file_id.clone();
            let name = job.path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            tauri::async_runtime::spawn_blocking(move || search.index_filename(&drive, &file, &name))
                .await
                .map_err(AppError::internal)??;
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
        self.queue_files_with_collections(paths, process_type, Vec::new()).await
    }

    async fn queue_files_with_collections(&self, paths: Vec<PathBuf>, process_type: &str, names: Vec<String>) -> AppResult<BackgroundProcess> {
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
        if !names.is_empty() {
            sqlx::query("UPDATE background_processes SET collections = ?2 WHERE process_id = ?1")
                .bind(&process_id).bind(serde_json::to_string(&names).map_err(AppError::serialization)?)
                .execute(&self.queue_pool).await.map_err(AppError::database)?;
        }
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
        let file_type = file_type_from_path(&job.path);
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
            let files_directory = drive_storage_root(&drive, &self.system_metadata_root)
                .join(IMPORTED_FILES_DIRECTORY);
            let destination = files_directory.join(destination_name(&job));

            if destination.try_exists()? {
                let _metadata_guard = self.metadata_lock.lock().await;
                if std::fs::metadata(&destination)?.len() != source_metadata.len() {
                    return Err(AppError::internal(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "the import destination already exists with a different size",
                    )));
                }
                write_import_sidecar(&job, &destination)?;
                self.save_selected_collections(&job, &drive_storage_root(&drive, &self.system_metadata_root), &saved.drive_id, &destination).await?;
                let (file_count, app_used_bytes, counts) =
                    calculate_managed_statistics(&files_directory)?;
                saved.file_count = file_count;
                saved.app_used_bytes = app_used_bytes;
                saved.file_type_counts = counts;

                self.storage_repository
                    .update(&saved, |updated| {
                        write_drive_metadata(&drive, &self.system_metadata_root, updated)
                    })
                    .await?;
                drop(_metadata_guard);
                self.index_filename(&saved.drive_id, &job).await?;
                self.publish_for_image_processing(&destination).await?;
                return Ok(());
            }

            if !can_fit(&drive, &saved, file_size) {
                continue;
            }

            std::fs::create_dir_all(&files_directory)?;
            let temporary = files_directory.join(format!(".{}.importing", job.file_id));
            // Keep imports serialized, but let browsing proceed while bytes are copied.
            let source = job.path.clone();
            let staging_path = temporary.clone();
            let expected_size = source_metadata.len();
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

            // Publish only a complete file. Readers share this short publication lock,
            // never the lock that serializes the potentially slow copy.
            let _metadata_guard = self.metadata_lock.lock().await;
            std::fs::rename(&temporary, &destination)?;
            write_import_sidecar(&job, &destination)?;
                self.save_selected_collections(&job, &drive_storage_root(&drive, &self.system_metadata_root), &saved.drive_id, &destination).await?;
            let mut counts = saved.file_type_counts.clone();
            let category = counts
                .iter_mut()
                .find(|entry| entry.file_type == file_type)
                .expect("all file types are represented");
            category.count = category
                .count
                .checked_add(1)
                .ok_or_else(|| AppError::internal(CounterOverflow))?;
            saved.file_count = saved
                .file_count
                .checked_add(1)
                .ok_or_else(|| AppError::internal(CounterOverflow))?;
            saved.app_used_bytes = saved
                .app_used_bytes
                .checked_add(file_size)
                .ok_or_else(|| AppError::internal(CounterOverflow))?;
            saved.file_type_counts = counts;

            self.storage_repository
                .update(&saved, |updated| {
                    write_drive_metadata(&drive, &self.system_metadata_root, updated)
                })
                .await?;
            drop(_metadata_guard);
            self.index_filename(&saved.drive_id, &job).await?;
            self.publish_for_image_processing(&destination).await?;
            return Ok(());
        }

        Err(AppError::storage_unavailable(
            "No mounted drive has enough available space for this file.",
        ))
    }

    async fn publish_for_image_processing(&self, path: &Path) -> AppResult<()> {
        if !is_supported_image(path) {
            return Ok(());
        }
        let mut queue = SqliteStorage::<ImageProcessingJob, (), ()>::new_in_queue(
            &self.queue_pool,
            IMAGE_PROCESSING_QUEUE,
        );
        queue
            .push(ImageProcessingJob {
                path: path.to_path_buf(),
            })
            .await
            .map_err(AppError::database)
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
    let temporary = path.with_file_name(format!(".{}.sidecar.tmp", job.file_id));
    let bytes = serde_json::to_vec(&ManagedFileMetadata {
        version: 1,
        name,
        collection_ids: Vec::new(),
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

fn copy_to_temporary(
    source: &Path,
    temporary: &Path,
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
    Ok(())
}

#[cfg(all(test, target_os = "windows"))]
mod concurrency_tests {
    use super::*;
    use crate::services::storage_service::StorageService;

    #[tokio::test]
    async fn stages_copy_while_browsing_holds_metadata_lock() {
        let root = std::env::temp_dir().join(format!("nexfile-copy-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("notes.txt");
        std::fs::write(&source, b"complete contents").unwrap();
        let database = SqliteDatabase::open(root.join("test.sqlite3")).await.unwrap();
        let storage = StorageService::new(SqliteStorageRepository::new(database.clone()), root.clone());
        let partition = storage.get_storage_data().await.unwrap().drives
            .into_iter().find(|drive| drive.is_system).unwrap().partition_name;
        storage.mount_drive(None, &partition).await.unwrap();
        let imports = ImportService::new(
            SqliteBackgroundProcessingRepository::new(database.clone()),
            SqliteStorageRepository::new(database.clone()), &database, root.clone(),
        ).await.unwrap();
        let catalog = crate::repositories::collection_repository::save(database.pool(), None, "Travel").await.unwrap();
        assert_eq!(catalog[0].id.len(), 14);
        assert!(!root.join("nexfile/collections.json").exists());
        assert!(crate::repositories::collection_repository::save(database.pool(), None, "travel").await.is_err());
        sqlx::query("INSERT INTO background_processes (process_id, process_type, status, collections) VALUES (?1, 'import_file', 'queued', ?2)").bind("test").bind("[\"Travel\"]").execute(database.pool()).await.unwrap();
        let guard = imports.metadata_lock().lock().await;
        let worker = imports.clone();
        let task = tauri::async_runtime::spawn(async move {
            worker.consume(ImportFileJob {
                process_id: "test".into(), file_id: "abcdefghijklmn".into(), path: source,
            }).await
        });
        let directory = root.join("nexfile").join("files");
        let staged = directory.join(".abcdefghijklmn.importing");
        let destination = directory.join("abcdefghijklmn.txt");
        let copied = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                if std::fs::read(&staged).ok().as_deref() == Some(b"complete contents") {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }).await;
        assert!(copied.is_ok(), "copy must proceed while a browser holds the metadata lock");
        assert!(!destination.exists(), "staged bytes must not be published yet");
        let page = storage.fetch_files(0, 60, None).await.unwrap();
        assert!(page.files.is_empty(), "browsing must exclude the staged file");
        drop(guard);
        task.await.unwrap().unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"complete contents");
        assert!(!staged.exists());
        let metadata = super::super::collection_service::read(&root.join("nexfile"), &storage.get_storage_data().await.unwrap().drives.into_iter().find(|drive| drive.is_system).unwrap().drive_id).unwrap();
        assert_eq!(metadata.collections[0].name, "Travel");
        assert_eq!(super::super::file_service::sidecar_collection_ids(&classification_output_path(&destination)).unwrap(), vec![metadata.collections[0].id.clone()]);
        assert_ne!(metadata.collections[0].id, catalog[0].id);
        assert_eq!(storage.fetch_files(0, 60, None).await.unwrap().files.len(), 1);
        imports.close().await;
        storage.close().await;
        std::fs::remove_dir_all(root).unwrap();
    }
}






pub(crate) fn assign_import_collections(root: &Path, drive_id: &str, destination: &Path, names: &[String]) -> AppResult<()> {
    if names.is_empty() { return Ok(()); }
    let mut metadata = super::collection_service::read(root, drive_id)?;
    let count = metadata.collections.len();
    let mut ids = Vec::new();
    for name in names {
        let existing = metadata.collections.iter().find(|item| item.name.to_lowercase() == name.trim().to_lowercase()).map(|item| item.id.clone());
        ids.push(match existing { Some(id) => id, None => metadata.create(name)? });
    }
    // Definitions must exist before the file starts referencing them.
    if metadata.collections.len() != count { super::collection_service::save(root, &metadata)?; }
    super::file_service::add_sidecar_collections(destination, &ids)
}
