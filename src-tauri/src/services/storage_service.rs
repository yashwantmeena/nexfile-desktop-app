use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::mappers::storage::{disconnected_drive, merge_connected_drive, storage_data};
use crate::models::storage::{DriveInfo, DriveMetadata, StorageData};
use crate::repositories::storage::RedbStorageRepository;
use crate::system::filesystem::{get_drives, read_file, write_file};

const METADATA_DIRECTORY: &str = "nexfile";
const METADATA_FILE: &str = "drive_metadata.json";

pub struct StorageService {
    repository: RedbStorageRepository,
    system_metadata_root: PathBuf,
}

impl StorageService {
    pub fn new(repository: RedbStorageRepository, system_metadata_root: PathBuf) -> Self {
        Self {
            repository,
            system_metadata_root,
        }
    }

    pub fn get_storage_data(&self) -> AppResult<StorageData> {
        let saved_drives = self
            .repository
            .list()?
            .into_iter()
            .map(|drive| (drive.drive_id.clone(), drive))
            .collect::<HashMap<_, _>>();

        let connected_drives = get_drives();

        let mut matched_drive_ids = HashSet::new();
        let mut drives = Vec::new();

        for drive in connected_drives {
            let metadata = read_drive_metadata(&drive, &self.system_metadata_root);
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

    pub fn mount_drive(
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

        let saved_drives = self.repository.list()?;
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
        let previous_drive_id = saved.map(|saved| saved.drive_id.clone());
        let metadata = metadata_for_mount(&drive, saved, metadata, next_priority);
        let metadata_file_path = metadata_path(&drive, &self.system_metadata_root);

        write_metadata(&metadata_file_path, &metadata)?;
        self.repository
            .replace(previous_drive_id.as_deref(), &metadata)?;

        self.get_storage_data()
    }

    pub fn unmount_drive(&self, drive_id: &str) -> AppResult<StorageData> {
        let drive_id = drive_id.trim();
        if drive_id.is_empty() {
            return Err(AppError::validation("A drive ID is required."));
        }

        let mut drive = self
            .repository
            .list()?
            .into_iter()
            .find(|drive| drive.drive_id == drive_id)
            .ok_or_else(|| AppError::validation("The selected drive is not saved."))?;

        drive.is_mounted = false;
        self.repository.save(&drive)?;
        self.get_storage_data()
    }

    pub fn remove_drive(&self, drive_id: &str) -> AppResult<StorageData> {
        let drive_id = drive_id.trim();
        if drive_id.is_empty() {
            return Err(AppError::validation("A drive ID is required."));
        }

        if !self.repository.delete(drive_id)? {
            return Err(AppError::validation("The selected drive is not saved."));
        }

        self.get_storage_data()
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
    next_priority: u32,
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
            app_limit_bytes: None,
            file_count: 0,
            app_used_bytes: 0,
            priority: next_priority,
            is_mounted: true,
        },
    };

    if metadata.drive_name.trim().is_empty() {
        metadata.drive_name = drive.drive_name.clone();
    }
    if metadata.partition_name.trim().is_empty() {
        metadata.partition_name = drive.partition_name.clone();
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

fn metadata_path(drive: &DriveInfo, system_metadata_root: &Path) -> PathBuf {
    let root = if drive.is_system {
        system_metadata_root
    } else {
        drive.mount_point.as_path()
    };

    root.join(METADATA_DIRECTORY).join(METADATA_FILE)
}

fn read_metadata(path: &Path) -> Option<DriveMetadata> {
    let encoded = read_file(path).ok()?;
    serde_json::from_slice(&encoded).ok()
}

fn read_drive_metadata(drive: &DriveInfo, system_metadata_root: &Path) -> Option<DriveMetadata> {
    read_metadata(&metadata_path(drive, system_metadata_root))
}

fn write_metadata(path: &Path, metadata: &DriveMetadata) -> AppResult<()> {
    let encoded = serde_json::to_vec_pretty(metadata).map_err(AppError::serialization)?;
    write_file(path, encoded).map_err(Into::into)
}
