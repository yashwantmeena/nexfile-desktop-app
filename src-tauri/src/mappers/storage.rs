use crate::models::storage::{DriveInfo, DriveMetadata, StorageData, StorageDrive};
use crate::utils::percentage;

pub(crate) fn storage_data(drives: Vec<StorageDrive>) -> StorageData {
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

pub(crate) fn merge_connected_drive(
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
        device_id: None,
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

pub(crate) fn disconnected_drive(drive: DriveMetadata) -> StorageDrive {
    let app_used_percent = drive
        .app_limit_bytes
        .map(|limit| percentage(drive.app_used_bytes, limit));

    StorageDrive {
        drive_id: drive.drive_id,
        device_id: None,
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
