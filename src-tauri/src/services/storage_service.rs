use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult, CounterOverflow};
use crate::mappers::storage_mapper::{disconnected_drive, merge_connected_drive, storage_data};

use crate::models::storage_model::{
    DriveConfigurationUpdate, DriveInfo, DriveMetadata, StorageData,
};
use crate::repositories::storage_repository::SqliteStorageRepository;
use crate::system::filesystem::{get_drives, read_file, write_file};
use crate::utils::constants::{DRIVE_METADATA_FILE, IMPORTED_FILES_DIRECTORY, NEXFILE_DIRECTORY};

#[derive(Clone)]
pub struct StorageService {
    repository: SqliteStorageRepository,
    system_metadata_root: PathBuf,
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
        favorite: Option<bool>,
        is_trashed: Option<bool>,
    ) -> AppResult<(PathBuf, String, String, Vec<String>, bool)> {
        let root = self.system_metadata_root.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let (drive, drive_metadata) = get_drives()
                .into_iter()
                .filter_map(|drive| {
                    let metadata = read_drive_metadata(&drive, &root)?;
                    (metadata.drive_id == drive_id).then_some((drive, metadata))
                })
                .next()
                .ok_or_else(|| AppError::storage_unavailable("The file's drive is not currently connected."))?;
            let files_directory = drive_storage_root(&drive, &root).join(IMPORTED_FILES_DIRECTORY);
            let canonical_files_directory = files_directory.canonicalize()?;
            let canonical_path = path.canonicalize()?;
            if canonical_path.parent() != Some(canonical_files_directory.as_path()) {
                return Err(AppError::validation("The selected file is outside NexFile's managed files."));
            }
            let sidecar = crate::services::image_processing_service::classification_output_path(&canonical_path);
            if !sidecar.is_file() {
                return Err(AppError::validation("The selected file has no metadata."));
            }
            let labels_updated = name.is_some()
                || category.is_some()
                || tags.is_some()
                || collection_names.is_some();
            if labels_updated
                && (name.is_none()
                    || category.is_none()
                    || tags.is_none()
                    || collection_names.is_none())
            {
                return Err(AppError::validation(
                    "Name, category, tags, and collections must be updated together.",
                ));
            }
            if labels_updated {
                let sidecar_bytes = std::fs::read(&sidecar)?;
                serde_json::from_slice::<crate::models::image_processing_model::ImageProcessingOutput>(&sidecar_bytes)
                    .map_err(|_| AppError::validation("Image analysis is still in progress. Try again shortly."))?;
            }

            let storage_root = drive_storage_root(&drive, &root);
            let mut collection_metadata = crate::services::collection_service::read(&storage_root, &drive_metadata.drive_id)?;
            let mut resolved_collection_ids = None;
            let mut definitions_changed = false;
            if let Some(collection_names) = collection_names {
                let mut collection_ids = Vec::with_capacity(collection_names.len());
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
                        collection_ids.push(collection.id.clone());
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
                tags.as_deref(),
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
                labels_updated,
            ))
        })
        .await
        .map_err(AppError::internal)?
    }

    pub async fn search_files(
        &self, index: crate::repositories::indexing_repository::TantivyIndexingRepository,
        query: String, mode: String, tags: Vec<String>, offset: usize, limit: usize,
        media_type: Option<crate::types::file_type::FileType>, collection: Option<String>,
        favorite_only: bool, trash_only: bool, model_category: Option<String>,
    ) -> AppResult<crate::models::file_model::FilePage> {
        if !(1..=200).contains(&limit) {
            return Err(AppError::validation("The file page size must be between 1 and 200."));
        }
        let snapshots = self.repository.list().await?;
        let root = self.system_metadata_root.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let connected = get_drives();
            if trash_only {
                return Ok(crate::services::file_service::fetch_matching_files_with_trash(
                    snapshots, connected, &root, media_type, offset, limit, None, true,
                ));
            }
            let collection_ids = collection
                .as_deref()
                .filter(|name| !name.trim().is_empty())
                .map(|name| {
                    crate::services::collection_service::ids_by_name(
                        &snapshots,
                        &connected,
                        &root,
                        name,
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
                    files: Vec::new(), total_count: 0, next_offset: None, issues: Vec::new(),
                });
            }
            Ok(crate::services::file_service::fetch_matching_files_with_trash(
                snapshots, connected, &root, media_type, offset, limit, Some(&matches), false))
        }).await.map_err(AppError::internal)?
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
        let snapshots = self.repository.list().await?;
        let root = self.system_metadata_root.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let connected = get_drives();
            if trash_only {
                let page = crate::services::file_service::fetch_matching_files_with_trash(
                    snapshots, connected, &root, None, 0, usize::MAX, None, true,
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
                        &snapshots,
                        &connected,
                        &root,
                        name,
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
                snapshots, connected, &root, None, 0, usize::MAX, Some(&matches), false,
            );
            let mut counts = crate::types::file_type::FileType::ALL
                .into_iter()
                .map(|file_type| crate::models::file_model::FileTypeCount { file_type, count: 0 })
                .collect::<Vec<_>>();
            for file in &page.files {
                if let Some(entry) = counts.iter_mut().find(|entry| entry.file_type == file.file_type) {
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
        .map_err(AppError::internal)?
    }
    pub async fn fetch_files(
        &self,
        offset: usize,
        limit: usize,
        media_type: Option<crate::types::file_type::FileType>,
    ) -> AppResult<crate::models::file_model::FilePage> {
        if !(1..=200).contains(&limit) {
            return Err(AppError::validation(
                "The file page size must be between 1 and 200.",
            ));
        }
        let snapshots = self.repository.list().await?;
        let root = self.system_metadata_root.clone();
        tauri::async_runtime::spawn_blocking(move || {
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
        .map_err(AppError::internal)
    }

    pub async fn get_file_count(&self) -> AppResult<crate::models::file_model::FileCountSummary> {
        let snapshot = self.repository.list().await?;
        let root = self.system_metadata_root.clone();
        tauri::async_runtime::spawn_blocking(move || {
            crate::services::file_service::verify_file_counts(snapshot, get_drives(), &root)
        })
        .await
        .map_err(AppError::internal)
    }

    /// Permanently remove one queued Trash item after confirming it is still soft-deleted.
    pub async fn permanently_delete_file(
        &self,
        drive_id: String,
        file_id: String,
        path: PathBuf,
    ) -> AppResult<()> {
        let root = self.system_metadata_root.clone();
        let (drive, metadata, file_count, app_used_bytes) = tauri::async_runtime::spawn_blocking(move || {
            let (drive, mut metadata) = get_drives()
                .into_iter()
                .filter_map(|drive| {
                    let metadata = read_drive_metadata(&drive, &root)?;
                    (metadata.drive_id == drive_id).then_some((drive, metadata))
                })
                .next()
                .ok_or_else(|| AppError::storage_unavailable("The file's drive is not currently connected."))?;
            let files_directory = drive_storage_root(&drive, &root).join(IMPORTED_FILES_DIRECTORY);
            let canonical_files_directory = files_directory.canonicalize()?;
            let parent = path.parent().ok_or_else(|| AppError::validation("The queued file has no parent folder."))?.canonicalize()?;
            if parent != canonical_files_directory {
                return Err(AppError::validation("The queued file is outside NexFile's managed files."));
            }
            if path.file_stem().and_then(|value| value.to_str()) != Some(file_id.as_str()) {
                return Err(AppError::validation("The queued file ID does not match its path."));
            }
            let sidecar = crate::services::image_processing_service::classification_output_path(&path);
            let file_exists = path.try_exists()?;
            if file_exists && !sidecar.is_file() {
                return Err(AppError::validation("The queued file no longer has its Trash metadata."));
            }
            if sidecar.is_file() {
                let sidecar_metadata = crate::services::file_service::read_managed_file_metadata(&sidecar)?;
                if !sidecar_metadata.is_trashed {
                    return Err(AppError::validation("The queued file was restored from Trash."));
                }
                if !sidecar_metadata.is_deleted {
                    crate::services::file_service::replace_sidecar_metadata(
                        &sidecar, None, None, None, None, None, None, Some(true),
                    )?;
                }
            }
            if file_exists {
                std::fs::remove_file(&path)?;
            }
            if sidecar.try_exists()? {
                std::fs::remove_file(&sidecar)?;
            }
            let (file_count, app_used_bytes) = calculate_managed_statistics(&canonical_files_directory)?;
            metadata.file_count = file_count;
            metadata.app_used_bytes = app_used_bytes;
            Ok((drive, metadata, file_count, app_used_bytes))
        })
        .await
        .map_err(AppError::internal)??;
        debug_assert_eq!(metadata.file_count, file_count);
        debug_assert_eq!(metadata.app_used_bytes, app_used_bytes);
        self.repository
            .update(&metadata, |updated| write_drive_metadata(&drive, &self.system_metadata_root, updated))
            .await?;
        Ok(())
    }

    pub fn new(repository: SqliteStorageRepository, system_metadata_root: PathBuf) -> Self {
        Self {
            repository,
            system_metadata_root,
        }
    }

    pub async fn get_storage_data(&self) -> AppResult<StorageData> {
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

        Ok(storage_data(drives))
    }

    pub async fn mount_drive(
        &self,
        device_id: Option<&str>,
        partition_name: &str,
    ) -> AppResult<StorageData> {
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

        self.get_storage_data().await
    }

    pub async fn unmount_drive(&self, drive_id: &str) -> AppResult<StorageData> {
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
        self.get_storage_data().await
    }

    pub async fn update_drive_configuration(
        &self,
        updates: &[DriveConfigurationUpdate],
    ) -> AppResult<StorageData> {
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
        self.get_storage_data().await
    }

    pub async fn remove_drive(&self, drive_id: &str) -> AppResult<StorageData> {
        let drive_id = drive_id.trim();
        if drive_id.is_empty() {
            return Err(AppError::validation("A drive ID is required."));
        }

        if !self.repository.delete(drive_id).await? {
            return Err(AppError::validation("The selected drive is not saved."));
        }

        self.get_storage_data().await
    }

    pub async fn close(&self) {
        self.repository.close().await;
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

pub(crate) fn calculate_managed_statistics(
    files_directory: &Path,
) -> AppResult<(i64, i64)> {

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

