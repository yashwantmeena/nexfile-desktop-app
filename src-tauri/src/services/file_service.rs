use crate::mappers::file_mapper::normalize_counts;
use std::path::Path;

use crate::models::file_model::{FileCountIssue, FileCountSummary, FileTypeCount};
use crate::models::storage_model::{DriveInfo, DriveMetadata};
use crate::services::storage_service::drive_storage_root;
use crate::utils::constants::DRIVE_METADATA_FILE;

pub(crate) fn verify_file_counts(
    snapshots: Vec<DriveMetadata>,
    connected: Vec<DriveInfo>,
    system_metadata_root: &Path,
) -> FileCountSummary {
    let metadata = connected
        .into_iter()
        .map(|drive| {
            let path = drive_storage_root(&drive, system_metadata_root).join(DRIVE_METADATA_FILE);
            let data = std::fs::read(path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<DriveMetadata>(&bytes).ok());
            (drive.partition_name, data)
        })
        .collect::<Vec<_>>();
    let mut counts = normalize_counts(Vec::new()).expect("empty counts are valid");
    let mut total = 0_i64;
    let mut issues = Vec::new();

    for saved in snapshots {
        let matches = metadata
            .iter()
            .filter(|(_, data)| {
                data.as_ref()
                    .is_some_and(|data| data.drive_id == saved.drive_id)
            })
            .collect::<Vec<_>>();
        let message = match matches.as_slice() {
            [(_, Some(data))] => validate_drive_counts(&saved, data),
            [] if metadata.iter().any(|(partition, _)| partition == &saved.partition_name) => {
                Some("Drive metadata is missing, unreadable, incomplete, or has a different drive ID. Counts cannot be verified.")
            }
            [] => Some("Drive is unavailable. Reconnect it to verify its counts."),
            _ => Some("Multiple metadata files claim the same drive ID. Counts cannot be verified."),
        };
        let message = message.or_else(|| {
            let Some(next_total) = total.checked_add(saved.file_count) else {
                return Some("File counts exceed the supported range.");
            };
            let Some(next_counts) = add_counts(&counts, &saved.file_type_counts) else {
                return Some("File counts exceed the supported range.");
            };
            total = next_total;
            counts = next_counts;
            None
        });
        if let Some(message) = message {
            issues.push(FileCountIssue {
                drive_id: saved.drive_id,
                drive_name: saved.drive_name,
                message: message.to_owned(),
            });
        }
    }
    let verified = issues.is_empty();
    FileCountSummary {
        counts: verified.then_some(counts),
        total_count: verified.then_some(total),
        issues,
    }
}

pub(crate) fn validate_drive_counts(
    saved: &DriveMetadata,
    data: &DriveMetadata,
) -> Option<&'static str> {
    if saved.file_type_counts != data.file_type_counts || saved.file_count != data.file_count {
        return Some("File counts in SQLite and drive metadata do not match. Data may be out of sync or modified; no counts have been changed.");
    }
    let values = saved
        .file_type_counts
        .iter()
        .map(|entry| entry.count)
        .collect::<Vec<_>>();
    if values.iter().any(|value| *value < 0)
        || values.into_iter().try_fold(0_i64, i64::checked_add) != Some(saved.file_count)
    {
        return Some("Category counts do not add up to the drive total. Counts may be incomplete or modified.");
    }
    None
}

pub(crate) fn add_counts(a: &[FileTypeCount], b: &[FileTypeCount]) -> Option<Vec<FileTypeCount>> {
    let mut result = a.to_vec();
    for entry in b {
        if let Some(existing) = result
            .iter_mut()
            .find(|existing| existing.file_type == entry.file_type)
        {
            existing.count = existing.count.checked_add(entry.count)?;
        } else {
            result.push(*entry);
        }
    }
    Some(result)
}
