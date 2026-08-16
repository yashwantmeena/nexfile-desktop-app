use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::error::AppResult;
use crate::models::storage::{DriveInfo, DriveMetadata, StorageData, StorageDrive};
use crate::repositories::storage::RedbStorageRepository;
use crate::system::filesystem::{get_drives, read_file};

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
            let metadata = read_metadata(&metadata_path(&drive, &self.system_metadata_root));
            let saved = metadata
                .as_ref()
                .and_then(|metadata| saved_drives.get(&metadata.drive_id));
            // A drive is mounted only when it is connected, identifiable by its
            // on-drive metadata, and enabled in the saved configuration.
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
}

fn storage_data(drives: Vec<StorageDrive>) -> StorageData {
    let connected_drives = drives.iter().filter(|drive| drive.is_connected);

    StorageData {
        total_bytes: connected_drives
            .clone()
            .map(|drive| drive.total_bytes)
            .sum(),
        available_bytes: connected_drives
            .clone()
            .filter_map(|drive| drive.available_bytes)
            .sum(),
        drives_detected: connected_drives.count(),
        file_indexed: drives.iter().map(|drive| drive.file_count).sum(),
        app_limit_bytes: drives
            .iter()
            .filter_map(|drive| drive.app_limit_bytes)
            .sum(),
        app_used_bytes: drives.iter().filter_map(|drive| drive.app_used_bytes).sum(),
        drives,
    }
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

fn merge_connected_drive(
    drive: DriveInfo,
    metadata: Option<&DriveMetadata>,
    saved: Option<&DriveMetadata>,
    is_mounted: bool,
) -> StorageDrive {
    let app_used_bytes = metadata.map(|metadata| metadata.app_used_bytes);
    let app_limit_bytes = metadata.and_then(|metadata| metadata.app_limit_bytes);

    StorageDrive {
        drive_id: metadata
            .map(|metadata| metadata.drive_id.clone())
            .unwrap_or_default(),
        drive_name: metadata
            .filter(|metadata| !metadata.drive_name.trim().is_empty())
            .map(|metadata| metadata.drive_name.clone())
            .unwrap_or(drive.drive_name),
        partition_name: metadata
            .filter(|metadata| !metadata.partition_name.trim().is_empty())
            .map(|metadata| metadata.partition_name.clone())
            .unwrap_or(drive.partition_name),
        file_system: drive.file_system,
        total_bytes: drive.total_bytes,
        system_used_bytes: Some(drive.system_used_bytes),
        system_used_percent: Some(percentage(drive.system_used_bytes, drive.total_bytes)),
        app_used_bytes,
        app_used_percent: app_used_bytes
            .zip(app_limit_bytes)
            .map(|(used, limit)| percentage(used, limit)),
        available_bytes: Some(drive.total_bytes.saturating_sub(drive.system_used_bytes)),
        app_limit_bytes,
        file_count: metadata.map_or(0, |metadata| metadata.file_count),
        priority: saved.map_or(0, |drive| drive.priority),
        is_mounted,
        is_connected: true,
        is_system: drive.is_system,
    }
}

fn disconnected_drive(drive: DriveMetadata) -> StorageDrive {
    let app_used_percent = drive
        .app_limit_bytes
        .map(|limit| percentage(drive.app_used_bytes, limit));

    StorageDrive {
        drive_id: drive.drive_id,
        drive_name: drive.drive_name,
        partition_name: drive.partition_name,
        file_system: String::new(),
        total_bytes: 0,
        system_used_bytes: None,
        system_used_percent: None,
        app_used_bytes: Some(drive.app_used_bytes),
        app_used_percent,
        available_bytes: None,
        app_limit_bytes: drive.app_limit_bytes,
        file_count: drive.file_count,
        priority: drive.priority,
        is_mounted: false,
        is_connected: false,
        is_system: false,
    }
}

fn percentage(value: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }

    let percent = (u128::from(value) * 100 / u128::from(total)).min(100);
    u8::try_from(percent).unwrap_or(100)
}
