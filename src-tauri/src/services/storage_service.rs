use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use futures::stream;

use crate::error::{AppError, AppResult, CounterOverflow};
use crate::mappers::storage_mapper::{disconnected_drive, merge_connected_drive, storage_data};

use crate::models::indexing_model::IndexingJob;
use crate::models::storage_model::{
    DriveConfigurationUpdate, DriveInfo, DriveMetadata, StorageData,
};
use crate::repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
use crate::repositories::database_repository::SqliteDatabase;
use crate::repositories::indexing_repository::TantivyIndexingRepository;
use crate::repositories::storage_repository::SqliteStorageRepository;
use crate::system::filesystem::{get_drives, read_file, write_file};
use crate::utils::constants::{
    DELETE_INDEX_PROCESS_TYPE, DRIVE_METADATA_FILE, IMPORTED_FILES_DIRECTORY,
    INDEXING_PROCESS_TYPE, INDEXING_QUEUE, INDEX_BATCH_SIZE, NEXFILE_DIRECTORY,
};
use crate::utils::operation_logger::log_event;

#[derive(Clone)]
pub struct StorageService {
    repository: SqliteStorageRepository,
    system_metadata_root: PathBuf,
    background_processing: Option<SqliteBackgroundProcessingRepository>,
    queue_pool: Option<sqlx::SqlitePool>,
    index: Option<TantivyIndexingRepository>,
}

impl StorageService {
    pub async fn update_file_metadata(
        &self,
        drive_id: String,
        path: PathBuf,
        name: Option<String>,
        category: Option<String>,
        tags: Option<Vec<String>>,
        collection_names: Option<Vec<String>>,
        add_tags: Vec<String>,
        add_collection_names: Vec<String>,
        favorite: Option<bool>,
        is_trashed: Option<bool>,
    ) -> AppResult<(
        PathBuf,
        String,
        String,
        Vec<String>,
        Vec<String>,
        bool,
        bool,
    )> {
        let path_subject = path.display().to_string();
        let drive_subject = drive_id.clone();
        log_event(
            "storage",
            "FILE-METADATA-START",
            format!("drive_id={drive_id} path={path_subject}"),
        );
        let root = self.system_metadata_root.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            let (drive, drive_metadata) = get_drives()
                .into_iter()
                .filter_map(|drive| {
                    let metadata = read_drive_metadata(&drive, &root)?;
                    (metadata.drive_id == drive_id).then_some((drive, metadata))
                })
                .next()
                .ok_or_else(|| {
                    AppError::storage_unavailable("The file's drive is not currently connected.")
                })?;
            let files_directory = drive_storage_root(&drive, &root).join(IMPORTED_FILES_DIRECTORY);
            let canonical_files_directory = files_directory.canonicalize()?;
            let canonical_path = path.canonicalize()?;
            if canonical_path.parent() != Some(canonical_files_directory.as_path()) {
                return Err(AppError::validation(
                    "The selected file is outside NexFile's managed files.",
                ));
            }
            let sidecar = crate::services::image_processing_service::classification_output_path(
                &canonical_path,
            );
            if !sidecar.is_file() {
                return Err(AppError::validation("The selected file has no metadata."));
            }
            let replaces_labels = name.is_some()
                || category.is_some()
                || tags.is_some()
                || collection_names.is_some();
            let labels_updated =
                replaces_labels || !add_tags.is_empty() || !add_collection_names.is_empty();
            if replaces_labels
                && (name.is_none()
                    || category.is_none()
                    || tags.is_none()
                    || collection_names.is_none())
            {
                return Err(AppError::validation(
                    "Name, category, tags, and collections must be updated together.",
                ));
            }
            if replaces_labels {
                let sidecar_bytes = std::fs::read(&sidecar)?;
                serde_json::from_slice::<
                    crate::models::image_processing_model::ImageProcessingOutput,
                >(&sidecar_bytes)
                .map_err(|_| {
                    AppError::validation("Image analysis is still in progress. Try again shortly.")
                })?;
            }

            let storage_root = drive_storage_root(&drive, &root);
            let current_metadata =
                crate::services::file_service::read_managed_file_metadata(&sidecar)?;
            let resolved_tags = if let Some(tags) = tags {
                Some(tags)
            } else if !add_tags.is_empty() {
                let mut tags = current_metadata.search_keywords.clone();
                for tag in add_tags {
                    if !tags
                        .iter()
                        .any(|existing| existing.eq_ignore_ascii_case(&tag))
                    {
                        tags.push(tag);
                    }
                }
                Some(tags)
            } else {
                None
            };
            let mut collection_metadata =
                crate::services::collection_service::read(&storage_root, &drive_metadata.drive_id)?;
            let mut resolved_collection_ids = None;
            let mut definitions_changed = false;
            let replaces_collections = collection_names.is_some();
            let collection_names = collection_names.unwrap_or(add_collection_names);
            if replaces_collections || !collection_names.is_empty() {
                let mut collection_ids = if replaces_collections {
                    Vec::with_capacity(collection_names.len())
                } else {
                    current_metadata.collection_ids.clone()
                };
                for name in collection_names {
                    let name = name.trim();
                    if name.is_empty() {
                        continue;
                    }
                    if let Some(collection) = collection_metadata
                        .collections
                        .iter()
                        .find(|collection| collection.name.eq_ignore_ascii_case(name))
                    {
                        if !collection_ids.contains(&collection.id) {
                            collection_ids.push(collection.id.clone());
                        }
                    } else {
                        let id = collection_metadata.create(name)?;
                        collection_ids.push(id);
                        definitions_changed = true;
                    }
                }
                resolved_collection_ids = Some(collection_ids);
            }
            if definitions_changed {
                crate::services::collection_service::save(&storage_root, &collection_metadata)?;
            }

            let metadata = crate::services::file_service::replace_sidecar_metadata(
                &sidecar,
                name.as_deref(),
                category.as_deref(),
                resolved_tags.as_deref(),
                resolved_collection_ids.as_deref(),
                favorite,
                is_trashed,
                None,
            )?;
            let file_id = canonical_path
                .file_stem()
                .and_then(|value| value.to_str())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| AppError::validation("The selected file has no file ID."))?
                .to_owned();
            Ok((
                sidecar,
                file_id,
                metadata.name,
                metadata.collection_ids,
                metadata.search_keywords,
                metadata.favorite,
                labels_updated,
            ))
        })
        .await
        .map_err(AppError::internal)?;
        if let Err(error) = &result {
            log_event(
                "storage",
                "FILE-METADATA-FAILED",
                format!("drive_id={drive_subject} path={path_subject} error={error}"),
            );
        } else {
            log_event(
                "storage",
                "FILE-METADATA-COMPLETE",
                format!("drive_id={drive_subject} path={path_subject}"),
            );
        }
        result
    }

    pub async fn search_files(
        &self,
        index: crate::repositories::indexing_repository::TantivyIndexingRepository,
        query: String,
        mode: String,
        tags: Vec<String>,
        offset: usize,
        limit: usize,
        media_type: Option<crate::types::file_type::FileType>,
        collection: Option<String>,
        favorite_only: bool,
        trash_only: bool,
        model_category: Option<String>,
    ) -> AppResult<crate::models::file_model::FilePage> {
        log_event(
            "storage",
            "SEARCH-START",
            format!(
                "query_length={} mode={mode} offset={offset} limit={limit} trash_only={trash_only}",
                query.chars().count()
            ),
        );
        if !(1..=200).contains(&limit) {
            return Err(AppError::validation(
                "The file page size must be between 1 and 200.",
            ));
        }
        let snapshots = self.repository.list().await?;
        let root = self.system_metadata_root.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            let connected = get_drives();
            if trash_only {
                return Ok(
                    crate::services::file_service::fetch_matching_files_with_trash(
                        snapshots, connected, &root, media_type, offset, limit, None, true,
                    ),
                );
            }
            let collection_ids = collection
                .as_deref()
                .filter(|name| !name.trim().is_empty())
                .map(|name| {
                    crate::services::collection_service::ids_by_name(
                        &snapshots, &connected, &root, name,
                    )
                })
                .transpose()?;
            let (matches, _) = index.indexed_results_filtered_with_category(
                &query,
                &mode,
                &tags,
                collection_ids.as_deref(),
                favorite_only,
                model_category.as_deref(),
            )?;
            if matches.is_empty() {
                return Ok(crate::models::file_model::FilePage {
                    files: Vec::new(),
                    total_count: 0,
                    next_offset: None,
                    issues: Vec::new(),
                });
            }
            Ok(
                crate::services::file_service::fetch_matching_files_with_trash(
                    snapshots,
                    connected,
                    &root,
                    media_type,
                    offset,
                    limit,
                    Some(&matches),
                    false,
                ),
            )
        })
        .await
        .map_err(AppError::internal)?;
        match &result {
            Ok(page) => log_event(
                "storage",
                "SEARCH-COMPLETE",
                format!(
                    "returned={} total={} issues={}",
                    page.files.len(),
                    page.total_count,
                    page.issues.len()
                ),
            ),
            Err(error) => log_event("storage", "SEARCH-FAILED", format!("error={error}")),
        }
        result
    }

    pub async fn search_file_count(
        &self,
        index: crate::repositories::indexing_repository::TantivyIndexingRepository,
        query: String,
        mode: String,
        tags: Vec<String>,
        collection: Option<String>,
        favorite_only: bool,
        trash_only: bool,
        model_category: Option<String>,
    ) -> AppResult<crate::models::file_model::FileCountSummary> {
        log_event(
            "storage",
            "COUNT-START",
            format!(
                "query_length={} mode={mode} trash_only={trash_only}",
                query.chars().count()
            ),
        );
        let snapshots = self.repository.list().await?;
        let root = self.system_metadata_root.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            let connected = get_drives();
            if trash_only {
                let page = crate::services::file_service::fetch_matching_files_with_trash(
                    snapshots,
                    connected,
                    &root,
                    None,
                    0,
                    usize::MAX,
                    None,
                    true,
                );
                return Ok(crate::models::file_model::FileCountSummary {
                    counts: None,
                    total_count: i64::try_from(page.total_count).ok(),
                    issues: page.issues,
                });
            }
            let collection_ids = collection
                .as_deref()
                .filter(|name| !name.trim().is_empty())
                .map(|name| {
                    crate::services::collection_service::ids_by_name(
                        &snapshots, &connected, &root, name,
                    )
                })
                .transpose()?;
            let (matches, _) = index.indexed_results_filtered_with_category(
                &query,
                &mode,
                &tags,
                collection_ids.as_deref(),
                favorite_only,
                model_category.as_deref(),
            )?;
            let page = crate::services::file_service::fetch_matching_files_with_trash(
                snapshots,
                connected,
                &root,
                None,
                0,
                usize::MAX,
                Some(&matches),
                false,
            );
            let mut counts = crate::types::file_type::FileType::ALL
                .into_iter()
                .map(|file_type| crate::models::file_model::FileTypeCount {
                    file_type,
                    count: 0,
                })
                .collect::<Vec<_>>();
            for file in &page.files {
                if let Some(entry) = counts
                    .iter_mut()
                    .find(|entry| entry.file_type == file.file_type)
                {
                    entry.count = entry.count.saturating_add(1);
                }
            }
            Ok(crate::models::file_model::FileCountSummary {
                counts: Some(counts),
                total_count: i64::try_from(page.total_count).ok(),
                issues: page.issues,
            })
        })
        .await
        .map_err(AppError::internal)?;
        match &result {
            Ok(summary) => log_event(
                "storage",
                "COUNT-COMPLETE",
                format!(
                    "total={:?} issues={}",
                    summary.total_count,
                    summary.issues.len()
                ),
            ),
            Err(error) => log_event("storage", "COUNT-FAILED", format!("error={error}")),
        }
        result
    }
    pub async fn fetch_files(
        &self,
        offset: usize,
        limit: usize,
        media_type: Option<crate::types::file_type::FileType>,
    ) -> AppResult<crate::models::file_model::FilePage> {
        log_event(
            "storage",
            "FETCH-START",
            format!("offset={offset} limit={limit} media_type={media_type:?}"),
        );
        if !(1..=200).contains(&limit) {
            return Err(AppError::validation(
                "The file page size must be between 1 and 200.",
            ));
        }
        let snapshots = self.repository.list().await?;
        let root = self.system_metadata_root.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            crate::services::file_service::fetch_files(
                snapshots,
                get_drives(),
                &root,
                media_type,
                offset,
                limit,
            )
        })
        .await
        .map_err(AppError::internal)?;
        match &result {
            page => log_event(
                "storage",
                "FETCH-COMPLETE",
                format!(
                    "returned={} total={} issues={}",
                    page.files.len(),
                    page.total_count,
                    page.issues.len()
                ),
            ),
        }
        Ok(result)
    }

    pub async fn get_file_count(&self) -> AppResult<crate::models::file_model::FileCountSummary> {
        log_event(
            "storage",
            "VERIFY-COUNT-START",
            "verifying managed file counts",
        );
        let snapshot = self.repository.list().await?;
        let root = self.system_metadata_root.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            crate::services::file_service::verify_file_counts(snapshot, get_drives(), &root)
        })
        .await
        .map_err(AppError::internal)?;
        match &result {
            summary => log_event(
                "storage",
                "VERIFY-COUNT-COMPLETE",
                format!(
                    "total={:?} issues={}",
                    summary.total_count,
                    summary.issues.len()
                ),
            ),
        }
        Ok(result)
    }

    /// Permanently remove one queued Trash item after confirming it is still soft-deleted.
    pub async fn permanently_delete_file(
        &self,
        drive_id: String,
        file_id: String,
        path: PathBuf,
    ) -> AppResult<()> {
        let path_subject = path.display().to_string();
        let drive_subject = drive_id.clone();
        let file_subject = file_id.clone();
        log_event(
            "storage",
            "PERMANENT-DELETE-START",
            format!("drive_id={drive_id} file_id={file_id} path={path_subject}"),
        );
        let root = self.system_metadata_root.clone();
        let (drive, metadata, file_count, app_used_bytes) =
            tauri::async_runtime::spawn_blocking(move || {
                let (drive, mut metadata) = get_drives()
                    .into_iter()
                    .filter_map(|drive| {
                        let metadata = read_drive_metadata(&drive, &root)?;
                        (metadata.drive_id == drive_id).then_some((drive, metadata))
                    })
                    .next()
                    .ok_or_else(|| {
                        AppError::storage_unavailable(
                            "The file's drive is not currently connected.",
                        )
                    })?;
                let files_directory =
                    drive_storage_root(&drive, &root).join(IMPORTED_FILES_DIRECTORY);
                let canonical_files_directory = files_directory.canonicalize()?;
                let parent = path
                    .parent()
                    .ok_or_else(|| AppError::validation("The queued file has no parent folder."))?
                    .canonicalize()?;
                if parent != canonical_files_directory {
                    return Err(AppError::validation(
                        "The queued file is outside NexFile's managed files.",
                    ));
                }
                if path.file_stem().and_then(|value| value.to_str()) != Some(file_id.as_str()) {
                    return Err(AppError::validation(
                        "The queued file ID does not match its path.",
                    ));
                }
                let sidecar =
                    crate::services::image_processing_service::classification_output_path(&path);
                let file_exists = path.try_exists()?;
                if file_exists && !sidecar.is_file() {
                    return Err(AppError::validation(
                        "The queued file no longer has its Trash metadata.",
                    ));
                }
                if sidecar.is_file() {
                    let sidecar_metadata =
                        crate::services::file_service::read_managed_file_metadata(&sidecar)?;
                    if !sidecar_metadata.is_trashed {
                        return Err(AppError::validation(
                            "The queued file was restored from Trash.",
                        ));
                    }
                    if !sidecar_metadata.is_deleted {
                        crate::services::file_service::replace_sidecar_metadata(
                            &sidecar,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            Some(true),
                        )?;
                    }
                }
                if file_exists {
                    std::fs::remove_file(&path)?;
                }
                if sidecar.try_exists()? {
                    std::fs::remove_file(&sidecar)?;
                }
                let (file_count, app_used_bytes) =
                    calculate_managed_statistics(&canonical_files_directory)?;
                metadata.file_count = file_count;
                metadata.app_used_bytes = app_used_bytes;
                Ok((drive, metadata, file_count, app_used_bytes))
            })
            .await
            .map_err(AppError::internal)??;
        debug_assert_eq!(metadata.file_count, file_count);
        debug_assert_eq!(metadata.app_used_bytes, app_used_bytes);
        self.repository
            .update(&metadata, |updated| {
                write_drive_metadata(&drive, &self.system_metadata_root, updated)
            })
            .await?;
        log_event(
            "storage",
            "PERMANENT-DELETE-COMPLETE",
            format!("drive_id={drive_subject} file_id={file_subject} path={path_subject}"),
        );
        Ok(())
    }

    pub fn new(repository: SqliteStorageRepository, system_metadata_root: PathBuf) -> Self {
        Self {
            repository,
            system_metadata_root,
            background_processing: None,
            queue_pool: None,
            index: None,
        }
    }

    /// Enables durable drive cleanup and reindexing jobs for the application runtime.
    pub fn with_drive_jobs(
        mut self,
        background_processing: SqliteBackgroundProcessingRepository,
        database: &SqliteDatabase,
        index: TantivyIndexingRepository,
    ) -> Self {
        self.background_processing = Some(background_processing);
        self.queue_pool = Some(database.pool().clone());
        self.index = Some(index);
        self
    }

    pub async fn get_storage_data(&self) -> AppResult<StorageData> {
        log_event("storage", "DRIVES-START", "refreshing drive state");
        let mut saved_drives = self
            .repository
            .list()
            .await?
            .into_iter()
            .map(|drive| (drive.drive_id.clone(), drive))
            .collect::<HashMap<_, _>>();

        let connected_drives = get_drives();

        let mut matched_drive_ids = HashSet::new();
        let mut drives = Vec::new();

        for drive in connected_drives {
            let mut metadata = read_drive_metadata(&drive, &self.system_metadata_root);
            if let Some(on_drive) = metadata.as_mut() {
                if let Some(saved) = saved_drives.get_mut(&on_drive.drive_id) {
                    if saved.is_mounted {
                        let files_directory =
                            drive_storage_root(&drive, &self.system_metadata_root)
                                .join(IMPORTED_FILES_DIRECTORY);
                        let (file_count, app_used_bytes) =
                            calculate_managed_statistics(&files_directory)?;
                        if saved.file_count != file_count
                            || saved.app_used_bytes != app_used_bytes
                            || on_drive.file_count != file_count
                            || on_drive.app_used_bytes != app_used_bytes
                        {
                            let mut reconciled = saved.clone();
                            reconciled.file_count = file_count;
                            reconciled.app_used_bytes = app_used_bytes;

                            let persisted = self
                                .repository
                                .update(&reconciled, |updated| {
                                    write_drive_metadata(
                                        &drive,
                                        &self.system_metadata_root,
                                        updated,
                                    )
                                })
                                .await?;
                            *saved = persisted.clone();
                            *on_drive = persisted;
                        }
                    }
                }
            }
            let saved = metadata
                .as_ref()
                .and_then(|metadata| saved_drives.get(&metadata.drive_id));
            let is_mounted = metadata.is_some() && saved.is_some_and(|drive| drive.is_mounted);

            if let Some(saved) = saved {
                matched_drive_ids.insert(saved.drive_id.clone());
            }

            drives.push(merge_connected_drive(
                drive,
                metadata.as_ref(),
                saved,
                is_mounted,
            ));
        }

        drives.extend(
            saved_drives
                .into_values()
                .filter(|drive| !matched_drive_ids.contains(&drive.drive_id))
                .map(disconnected_drive),
        );

        let data = storage_data(drives);
        log_event(
            "storage",
            "DRIVES-COMPLETE",
            format!(
                "connected={} detected={} mounted={}",
                data.drives
                    .iter()
                    .filter(|drive| drive.is_connected)
                    .count(),
                data.drives_detected,
                data.drives.iter().filter(|drive| drive.is_mounted).count()
            ),
        );
        Ok(data)
    }

    pub async fn mount_drive(
        &self,
        device_id: Option<&str>,
        partition_name: &str,
    ) -> AppResult<StorageData> {
        self.mount_drive_with_files_directory(device_id, partition_name)
            .await
            .map(|(storage, _)| storage)
    }

    pub(crate) async fn mount_drive_with_files_directory(
        &self,
        device_id: Option<&str>,
        partition_name: &str,
    ) -> AppResult<(StorageData, PathBuf)> {
        log_event(
            "storage",
            "MOUNT-START",
            format!("device_id={device_id:?} partition={partition_name}"),
        );
        let device_id = device_id.map(str::trim).filter(|value| !value.is_empty());
        let partition_name = partition_name.trim();
        if device_id.is_none() && partition_name.is_empty() {
            return Err(AppError::validation(
                "A device ID or partition name is required.",
            ));
        }

        let saved_drives = self.repository.list().await?;
        let drive = get_drives()
            .into_iter()
            .find(|drive| {
                is_requested_drive(drive, device_id, partition_name, &self.system_metadata_root)
            })
            .ok_or_else(|| {
                AppError::storage_unavailable("The selected drive is not currently connected.")
            })?;

        let metadata = read_drive_metadata(&drive, &self.system_metadata_root);

        let saved = metadata.as_ref().and_then(|metadata| {
            saved_drives
                .iter()
                .find(|saved| saved.drive_id == metadata.drive_id)
        });
        let next_priority = saved_drives
            .iter()
            .filter(|saved| saved.is_mounted)
            .map(|saved| saved.priority)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let was_saved = saved.is_some();
        let metadata = metadata_for_mount(&drive, saved, metadata, next_priority);
        let metadata_file_path = metadata_path(&drive, &self.system_metadata_root);

        if was_saved {
            self.repository
                .update(&metadata, |updated| {
                    write_metadata(&metadata_file_path, updated)
                })
                .await?;
        } else {
            let inserted = self.repository.insert(&metadata).await?;
            write_metadata(&metadata_file_path, &inserted)?;
        }

        let files_directory =
            drive_storage_root(&drive, &self.system_metadata_root).join(IMPORTED_FILES_DIRECTORY);
        self.publish_drive_index_create(files_directory.clone(), INDEXING_PROCESS_TYPE)
            .await?;
        log_event(
            "storage",
            "MOUNT-COMPLETE",
            format!(
                "drive_id={} files_root={}",
                metadata.drive_id,
                files_directory.display()
            ),
        );
        Ok((self.get_storage_data().await?, files_directory))
    }

    pub async fn unmount_drive(&self, drive_id: &str) -> AppResult<StorageData> {
        log_event("storage", "UNMOUNT-START", format!("drive_id={drive_id}"));
        let drive_id = drive_id.trim();
        if drive_id.is_empty() {
            return Err(AppError::validation("A drive ID is required."));
        }

        let mut drive = self
            .repository
            .get(drive_id)
            .await?
            .ok_or_else(|| AppError::validation("The selected drive is not saved."))?;

        drive.is_mounted = false;
        // Unmounting is a local database preference, including for offline drives.
        self.repository.update(&drive, |_| Ok(())).await?;
        let data = self.get_storage_data().await?;
        log_event(
            "storage",
            "UNMOUNT-COMPLETE",
            format!("drive_id={drive_id}"),
        );
        Ok(data)
    }

    pub async fn update_drive_configuration(
        &self,
        updates: &[DriveConfigurationUpdate],
    ) -> AppResult<StorageData> {
        log_event(
            "storage",
            "CONFIG-START",
            format!("drives={}", updates.len()),
        );
        if updates.is_empty() {
            return Err(AppError::validation(
                "At least one mounted drive is required.",
            ));
        }

        let mut unique_drive_ids = HashSet::new();
        for update in updates {
            let drive_id = update.drive_id.trim();
            if drive_id.is_empty() {
                return Err(AppError::validation("A drive ID is required."));
            }
            if !unique_drive_ids.insert(drive_id) {
                return Err(AppError::validation(
                    "Each drive can appear only once in the priority order.",
                ));
            }
            if update.app_limit_bytes.is_some_and(|limit| limit < 0) {
                return Err(AppError::validation(
                    "A drive storage limit cannot be negative.",
                ));
            }
        }

        let mut saved_drives = self.repository.list().await?;
        let mut updated_drives = Vec::with_capacity(updates.len());

        for (index, update) in updates.iter().enumerate() {
            let drive_id = update.drive_id.trim();
            let drive = saved_drives
                .iter_mut()
                .find(|drive| drive.drive_id == drive_id)
                .ok_or_else(|| AppError::validation("The selected drive is not saved."))?;

            if !drive.is_mounted {
                return Err(AppError::validation(
                    "Only mounted drives can have a storage priority.",
                ));
            }

            drive.priority = i64::try_from(index + 1)
                .map_err(|_| AppError::validation("Too many drives were provided."))?;
            drive.app_limit_bytes = update.app_limit_bytes;
            updated_drives.push(drive.clone());
        }

        let updated_by_id = updated_drives
            .iter()
            .map(|drive| (drive.drive_id.as_str(), drive))
            .collect::<HashMap<_, _>>();
        let mut metadata_updates = Vec::new();

        for connected_drive in get_drives() {
            let Some(mut metadata) =
                read_drive_metadata(&connected_drive, &self.system_metadata_root)
            else {
                continue;
            };
            let Some(updated) = updated_by_id.get(metadata.drive_id.as_str()) else {
                continue;
            };
            if updated
                .app_limit_bytes
                .is_some_and(|limit| limit > connected_drive.total_bytes)
            {
                return Err(AppError::validation(
                    "A drive storage limit cannot exceed its total capacity.",
                ));
            }

            metadata.priority = updated.priority;
            metadata.app_limit_bytes = updated.app_limit_bytes;
            metadata_updates.push((
                metadata_path(&connected_drive, &self.system_metadata_root),
                metadata,
            ));
        }

        for drive in &updated_drives {
            self.repository
                .update(drive, |persisted| {
                    for (path, metadata) in &metadata_updates {
                        if metadata.drive_id == persisted.drive_id {
                            let mut metadata = metadata.clone();
                            metadata.created_at_ms = persisted.created_at_ms;
                            metadata.updated_at_ms = persisted.updated_at_ms;
                            write_metadata(path, &metadata)?;
                        }
                    }
                    Ok(())
                })
                .await?;
        }
        let data = self.get_storage_data().await?;
        log_event(
            "storage",
            "CONFIG-COMPLETE",
            format!("drives={}", updated_drives.len()),
        );
        Ok(data)
    }

    pub async fn remove_drive(&self, drive_id: &str) -> AppResult<StorageData> {
        log_event("storage", "REMOVE-START", format!("drive_id={drive_id}"));
        let drive_id = drive_id.trim();
        if drive_id.is_empty() {
            return Err(AppError::validation("A drive ID is required."));
        }

        if !self.repository.delete(drive_id).await? {
            return Err(AppError::validation("The selected drive is not saved."));
        }

        self.publish_drive_index_cleanup(drive_id, DELETE_INDEX_PROCESS_TYPE)
            .await?;

        let data = self.get_storage_data().await?;
        log_event("storage", "REMOVE-COMPLETE", format!("drive_id={drive_id}"));
        Ok(data)
    }

    async fn publish_drive_index_cleanup(
        &self,
        drive_id: &str,
        process_type: &str,
    ) -> AppResult<()> {
        let (Some(repository), Some(queue_pool), Some(index)) =
            (&self.background_processing, &self.queue_pool, &self.index)
        else {
            return Ok(());
        };
        let drive_id = drive_id.to_owned();
        let file_ids = {
            let index = index.clone();
            let drive_id = drive_id.clone();
            tauri::async_runtime::spawn_blocking(move || index.file_ids_for_drive(&drive_id))
                .await
                .map_err(AppError::internal)??
        };
        let batches = file_ids
            .chunks(INDEX_BATCH_SIZE)
            .map(|file_ids| file_ids.to_vec())
            .collect::<Vec<_>>();
        log_event(
            INDEXING_QUEUE,
            "DISCOVERED",
            format!(
                "operation=delete-index process_type={process_type} drive_id={drive_id} indexed_files={} batches={}",
                file_ids.len(),
                batches.len()
            ),
        );
        self.publish_index_batches(
            repository,
            queue_pool,
            process_type,
            batches
                .into_iter()
                .map(|file_ids| IndexingJob::DeleteIndexBatch {
                    process_id: String::new(),
                    drive_id: drive_id.clone(),
                    file_ids,
                }),
        )
        .await
    }

    async fn publish_drive_index_create(
        &self,
        files_directory: PathBuf,
        process_type: &str,
    ) -> AppResult<()> {
        let (Some(repository), Some(queue_pool)) = (&self.background_processing, &self.queue_pool)
        else {
            return Ok(());
        };
        let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
        log_event(
            INDEXING_QUEUE,
            "SCAN",
            format!(
                "operation=index process_type={process_type} root={}",
                files_directory.display()
            ),
        );
        let scanner = tauri::async_runtime::spawn_blocking(move || {
            stream_managed_sidecar_batches(&files_directory, sender)
        });
        while let Some(paths) = receiver.recv().await {
            self.publish_index_batches(
                repository,
                queue_pool,
                process_type,
                std::iter::once(IndexingJob::CreateIndexBatch {
                    process_id: String::new(),
                    paths,
                }),
            )
            .await?;
        }
        scanner.await.map_err(AppError::internal)??;
        log_event(
            INDEXING_QUEUE,
            "SCAN-COMPLETE",
            format!("operation=index process_type={process_type}"),
        );
        Ok(())
    }

    async fn publish_index_batches<I>(
        &self,
        repository: &SqliteBackgroundProcessingRepository,
        queue_pool: &sqlx::SqlitePool,
        process_type: &str,
        batches: I,
    ) -> AppResult<()>
    where
        I: IntoIterator<Item = IndexingJob>,
    {
        let mut batches = batches.into_iter().collect::<Vec<_>>();
        let total_items = u64::try_from(batches.len())
            .map_err(|_| AppError::validation("Too many indexing batches."))?;
        if total_items == 0 {
            return Ok(());
        }
        let process = repository.acquire_stage(process_type, total_items).await?;
        for job in &mut batches {
            match job {
                IndexingJob::CreateIndexBatch { process_id, .. }
                | IndexingJob::DeleteIndexBatch { process_id, .. } => {
                    *process_id = process.process_id.clone();
                }
            }
        }
        for job in &batches {
            log_queued_index_batch(process_type, job);
        }
        let mut jobs = stream::iter(batches.into_iter().map(|job| Task::builder(job).build()));
        let mut queue =
            SqliteStorage::<IndexingJob, (), ()>::new_in_queue(queue_pool, INDEXING_QUEUE);
        if let Err(error) = queue.push_all(&mut jobs).await {
            let _ = repository
                .rollback_items(&process.process_id, total_items)
                .await;
            return Err(AppError::database(error));
        }
        Ok(())
    }

    pub async fn close(&self) {
        self.repository.close().await;
    }
}

fn log_queued_index_batch(process_type: &str, job: &IndexingJob) {
    match job {
        IndexingJob::CreateIndexBatch { process_id, paths } => log_event(
            INDEXING_QUEUE,
            "QUEUED",
            format!(
                "process_type={process_type} process_id={process_id} operation=index items={} cleanup=identity",
                paths.len()
            ),
        ),
        IndexingJob::DeleteIndexBatch {
            process_id,
            drive_id,
            file_ids,
        } => log_event(
            INDEXING_QUEUE,
            "QUEUED",
            format!(
                "process_type={process_type} process_id={process_id} operation=delete-index drive_id={drive_id} items={}",
                file_ids.len()
            ),
        ),
    }
}

fn is_requested_drive(
    drive: &DriveInfo,
    device_id: Option<&str>,
    partition_name: &str,
    system_metadata_root: &Path,
) -> bool {
    if device_id.is_some_and(|device_id| drive.device_id == device_id) {
        return true;
    }

    if partition_name.is_empty() {
        return false;
    }

    if drive.partition_name == partition_name {
        return true;
    }

    read_drive_metadata(drive, system_metadata_root)
        .is_some_and(|metadata| metadata.partition_name == partition_name)
}

fn metadata_for_mount(
    drive: &DriveInfo,
    saved: Option<&DriveMetadata>,
    file_metadata: Option<DriveMetadata>,
    next_priority: i64,
) -> DriveMetadata {
    let metadata_matches_saved =
        saved
            .zip(file_metadata.as_ref())
            .is_some_and(|(saved, metadata)| {
                saved.drive_id == metadata.drive_id
                    && saved.file_count == metadata.file_count
                    && saved.app_used_bytes == metadata.app_used_bytes
            });

    let mut metadata = match (metadata_matches_saved, saved, file_metadata) {
        (true, Some(saved), _) => {
            // The saved record and on-drive metadata agree, so mounting only
            // needs to update configuration fields.
            saved.clone()
        }
        (_, _, Some(metadata)) => {
            // The metadata UUID is authoritative. A missing/inconsistent
            // database record is recreated from it; indexing starts here later.
            metadata
        }
        _ => DriveMetadata {
            // A drive with no metadata is new to NexFile.
            drive_id: uuid::Uuid::new_v4().to_string(),
            drive_name: drive.drive_name.clone(),
            partition_name: drive.partition_name.clone(),
            app_limit_bytes: Some(drive.total_bytes),
            file_count: 0,
            app_used_bytes: 0,
            priority: next_priority,
            is_mounted: true,
            created_at_ms: 0,
            updated_at_ms: 0,
        },
    };

    if metadata.drive_name.trim().is_empty() {
        metadata.drive_name = drive.drive_name.clone();
    }
    if metadata.partition_name.trim().is_empty() {
        metadata.partition_name = drive.partition_name.clone();
    }
    if metadata.app_limit_bytes.is_none() {
        metadata.app_limit_bytes = Some(drive.total_bytes);
    }
    metadata.priority = if metadata_matches_saved {
        saved
            .filter(|saved| saved.priority > 0)
            .map_or(next_priority, |saved| saved.priority)
    } else {
        next_priority
    };
    metadata.is_mounted = true;
    metadata
}

pub(crate) fn drive_storage_root(drive: &DriveInfo, system_metadata_root: &Path) -> PathBuf {
    let root = if drive.is_system {
        system_metadata_root
    } else {
        drive.mount_point.as_path()
    };

    root.join(NEXFILE_DIRECTORY)
}

pub(crate) fn calculate_managed_statistics(files_directory: &Path) -> AppResult<(i64, i64)> {
    if !files_directory.try_exists()? {
        return Ok((0, 0));
    }

    let mut file_count = 0_i64;
    let mut app_used_bytes = 0_i64;
    for entry in std::fs::read_dir(files_directory)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if !metadata.is_file()
            || entry.file_name().to_string_lossy().starts_with('.')
            || is_generated_image_sidecar(&path)
        {
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

pub(crate) fn is_generated_image_sidecar(path: &Path) -> bool {
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        return false;
    }

    path.file_stem()
        .map(|original_name| path.with_file_name(original_name))
        .is_some_and(|original_path| original_path.is_file())
}

fn stream_managed_sidecar_batches(
    files_directory: &Path,
    sender: tokio::sync::mpsc::Sender<Vec<PathBuf>>,
) -> AppResult<()> {
    let mut sidecars = Vec::with_capacity(INDEX_BATCH_SIZE);
    let entries = match std::fs::read_dir(files_directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file()
            || entry.file_name().to_string_lossy().starts_with('.')
            || is_generated_image_sidecar(&path)
        {
            continue;
        }
        let sidecar = crate::services::image_processing_service::classification_output_path(&path);
        if sidecar.is_file()
            && crate::services::file_service::read_managed_file_metadata(&sidecar).is_ok()
        {
            sidecars.push(sidecar);
            if sidecars.len() == INDEX_BATCH_SIZE {
                let batch = std::mem::replace(&mut sidecars, Vec::with_capacity(INDEX_BATCH_SIZE));
                if sender.blocking_send(batch).is_err() {
                    return Ok(());
                }
            }
        }
    }
    if !sidecars.is_empty() {
        let _ = sender.blocking_send(sidecars);
    }
    Ok(())
}

fn metadata_path(drive: &DriveInfo, system_metadata_root: &Path) -> PathBuf {
    drive_storage_root(drive, system_metadata_root).join(DRIVE_METADATA_FILE)
}

pub(crate) fn read_metadata(path: &Path) -> Option<DriveMetadata> {
    let encoded = read_file(path).ok()?;
    serde_json::from_slice(&encoded).ok()
}

pub(crate) fn read_drive_metadata(
    drive: &DriveInfo,
    system_metadata_root: &Path,
) -> Option<DriveMetadata> {
    read_metadata(&metadata_path(drive, system_metadata_root))
}

pub(crate) fn write_drive_metadata(
    drive: &DriveInfo,
    system_metadata_root: &Path,
    metadata: &DriveMetadata,
) -> AppResult<()> {
    write_metadata(&metadata_path(drive, system_metadata_root), metadata)
}

pub(crate) fn write_metadata(path: &Path, metadata: &DriveMetadata) -> AppResult<()> {
    let encoded = serde_json::to_vec_pretty(metadata).map_err(AppError::serialization)?;
    write_file(path, encoded).map_err(Into::into)
}
