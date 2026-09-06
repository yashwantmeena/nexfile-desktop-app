use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult, CounterOverflow};
use crate::mappers::storage_mapper::{disconnected_drive, merge_connected_drive, storage_data};
use crate::models::file_model::FileTypeCount;
use crate::models::storage_model::{
    DriveConfigurationUpdate, DriveInfo, DriveMetadata, StorageData,
};
use crate::repositories::storage_repository::SqliteStorageRepository;
use crate::system::filesystem::{get_drives, read_file, write_file};
use crate::utils::constants::{DRIVE_METADATA_FILE, IMPORTED_FILES_DIRECTORY, NEXFILE_DIRECTORY};

pub struct StorageService {
    repository: SqliteStorageRepository,
    system_metadata_root: PathBuf,
}

impl StorageService {
    pub async fn search_files(
        &self, index: crate::repositories::indexing_repository::TantivyIndexingRepository,
        query: String, mode: String, tags: Vec<String>, offset: usize, limit: usize,
        media_type: Option<crate::types::file_type::FileType>,
    ) -> AppResult<crate::models::file_model::FilePage> {
        if !(1..=200).contains(&limit) {
            return Err(AppError::validation("The file page size must be between 1 and 200."));
        }
        let snapshots = self.repository.list().await?;
        let root = self.system_metadata_root.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let matches = index.search_files(&query, &mode, &tags)?;
            if matches.is_empty() {
                return Ok(crate::models::file_model::FilePage {
                    files: Vec::new(), total_count: 0, next_offset: None, issues: Vec::new(),
                });
            }
            Ok(crate::services::file_service::fetch_matching_files(
                snapshots, get_drives(), &root, media_type, offset, limit, Some(&matches)))
        }).await.map_err(AppError::internal)?
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
                        let (file_count, app_used_bytes, file_type_counts) =
                            calculate_managed_statistics(&files_directory)?;
                        if saved.file_count != file_count
                            || saved.app_used_bytes != app_used_bytes
                            || on_drive.file_count != file_count
                            || on_drive.app_used_bytes != app_used_bytes
                        {
                            let mut reconciled = saved.clone();
                            reconciled.file_count = file_count;
                            reconciled.app_used_bytes = app_used_bytes;
                            reconciled.file_type_counts = file_type_counts;
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

        let file_type_counts = saved_drives.values().try_fold(
            crate::mappers::file_mapper::normalize_counts(Vec::new())
                .expect("empty counts are valid"),
            |counts, drive| {
                crate::services::file_service::add_counts(&counts, &drive.file_type_counts)
                    .ok_or_else(|| AppError::internal(CounterOverflow))
            },
        )?;
        drives.extend(
            saved_drives
                .into_values()
                .filter(|drive| !matched_drive_ids.contains(&drive.drive_id))
                .map(disconnected_drive),
        );

        Ok(storage_data(drives, file_type_counts))
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
                    && saved.file_type_counts == metadata.file_type_counts
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
            file_type_counts: crate::mappers::file_mapper::normalize_counts(Vec::new())
                .expect("empty counts are valid"),
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
) -> AppResult<(i64, i64, Vec<FileTypeCount>)> {
    let mut counts =
        crate::mappers::file_mapper::normalize_counts(Vec::new()).map_err(AppError::validation)?;
    if !files_directory.try_exists()? {
        return Ok((0, 0, counts));
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
        let file_type = crate::mappers::file_mapper::file_type_from_path(&path);
        let entry_count = counts
            .iter_mut()
            .find(|entry| entry.file_type == file_type)
            .expect("all file types are represented");
        entry_count.count = entry_count
            .count
            .checked_add(1)
            .ok_or_else(|| AppError::internal(CounterOverflow))?;
        let size = i64::try_from(metadata.len()).map_err(AppError::internal)?;
        app_used_bytes = app_used_bytes
            .checked_add(size)
            .ok_or_else(|| AppError::internal(CounterOverflow))?;
    }
    Ok((file_count, app_used_bytes, counts))
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
