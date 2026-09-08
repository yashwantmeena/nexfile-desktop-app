use std::path::Path;

use crate::models::file_model::{FileCountIssue, FileCountSummary, FileTypeCount};
use crate::models::storage_model::{DriveInfo, DriveMetadata};
use crate::services::storage_service::drive_storage_root;
use crate::utils::constants::DRIVE_METADATA_FILE;

/// Lists managed files without requiring an AI sidecar or a search-index entry.
/// Each page is a fresh filesystem snapshot; imports/deletions can shift offsets.
pub(crate) fn fetch_files(
    snapshots: Vec<DriveMetadata>,
    connected: Vec<DriveInfo>,
    system_metadata_root: &Path,
    media_type: Option<crate::types::file_type::FileType>,
    offset: usize,
    limit: usize,
) -> crate::models::file_model::FilePage {
    fetch_matching_files(snapshots, connected, system_metadata_root, media_type, offset, limit, None)
}

pub(crate) fn fetch_matching_files(
    snapshots: Vec<DriveMetadata>,
    connected: Vec<DriveInfo>,
    system_metadata_root: &Path,
    media_type: Option<crate::types::file_type::FileType>,
    offset: usize,
    limit: usize,
    indexed_matches: Option<&std::collections::HashSet<(String, String)>>,
) -> crate::models::file_model::FilePage {
    use crate::models::file_model::{FetchedFile, FilePage};
    use crate::services::storage_service::{is_generated_image_sidecar, read_drive_metadata};
    use crate::utils::constants::IMPORTED_FILES_DIRECTORY;

    let connected = connected
        .into_iter()
        .map(|drive| {
            let metadata = read_drive_metadata(&drive, system_metadata_root);
            (drive, metadata)
        })
        .collect::<Vec<_>>();
    let mut files = Vec::new();
    let mut issues = Vec::new();
    for saved in snapshots {
        let matches = connected
            .iter()
            .filter(|(_, metadata)| {
                metadata
                    .as_ref()
                    .is_some_and(|metadata| metadata.drive_id == saved.drive_id)
            })
            .collect::<Vec<_>>();
        let issue = |message: &str| FileCountIssue {
            drive_id: saved.drive_id.clone(),
            drive_name: saved.drive_name.clone(),
            message: message.to_owned(),
        };
        let [(drive, _)] = matches.as_slice() else {
            issues.push(issue(if matches.is_empty() {
                "Drive is unavailable or its metadata could not be verified."
            } else {
                "Multiple connected drives claim the same drive ID."
            }));
            continue;
        };
        let directory =
            drive_storage_root(drive, system_metadata_root).join(IMPORTED_FILES_DIRECTORY);
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && saved.file_count == 0 => {
                continue
            }
            Err(_) => {
                issues.push(issue("The managed files folder is unavailable."));
                continue;
            }
        };
        let mut incomplete = false;
        for entry in entries {
            let Ok(entry) = entry else {
                incomplete = true;
                continue;
            };
            let path = entry.path();
            if let Some(matches) = indexed_matches {
                let file_id = path.file_stem().and_then(|value| value.to_str()).unwrap_or_default();
                if !matches.contains(&(saved.drive_id.clone(), file_id.to_owned())) {
                    continue;
                }
            }
            if entry.file_name().to_string_lossy().starts_with('.')
                || is_generated_image_sidecar(&path)
            {
                continue;
            }
            // Do not follow links outside the managed folder.
            let metadata = match std::fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_file() => metadata,
                Ok(_) => continue,
                Err(_) => {
                    incomplete = true;
                    continue;
                }
            };
            let modified_at_ms = metadata.modified().ok().and_then(|time| {
                match time.duration_since(std::time::UNIX_EPOCH) {
                    Ok(duration) => i64::try_from(duration.as_millis()).ok(),
                    Err(error) => i64::try_from(error.duration().as_millis())
                        .ok()
                        .map(|ms| -ms),
                }
            });
            let managed_name = entry.file_name().to_string_lossy().into_owned();
            let name = read_name(&path).unwrap_or_else(|| managed_name.clone());
            files.push(FetchedFile {
                id: format!("{}:{}", saved.drive_id, managed_name),
                drive_id: saved.drive_id.clone(),
                name,
                file_type: crate::mappers::file_mapper::file_type_from_path(&path),
                path,
                size_bytes: metadata.len(),
                modified_at_ms,
                categories: Vec::new(),
                tags: Vec::new(),
            });
        }
        if incomplete {
            issues.push(issue("Some files could not be read. Refresh to try again."));
        }
    }
    if let Some(media_type) = media_type {
        files.retain(|file| file.file_type == media_type);
    }
    files.sort_by(|a, b| {
        b.modified_at_ms
            .cmp(&a.modified_at_ms)
            .then_with(|| a.id.cmp(&b.id))
    });
    let total_count = files.len();
    let end = offset.saturating_add(limit).min(total_count);
    let next_offset = (end < total_count).then_some(end);
    let files = files
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|mut file| {
            if file.file_type == crate::types::file_type::FileType::Image {
                let (categories, tags) = read_file_labels(&file.path);
                file.categories = categories;
                file.tags = tags;
            }
            file
        })
        .collect();
    FilePage {
        files,
        total_count,
        next_offset,
        issues,
    }
}

fn read_name(file_path: &Path) -> Option<String> {
    let path = crate::services::image_processing_service::classification_output_path(file_path);
    let metadata = serde_json::from_slice::<crate::models::file_model::ManagedFileMetadata>(
        &std::fs::read(path).ok()?,
    )
    .ok()?;
    let name = metadata.name.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

fn read_file_labels(path: &Path) -> (Vec<String>, Vec<String>) {
    #[derive(serde::Deserialize)]
    struct Sidecar {
        classification: crate::models::image_processing_model::ImageClassificationOutput,
        #[serde(default, rename = "searchKeywords", alias = "search_keywords")]
        search_keywords: Vec<String>,
    }
    let sidecar = crate::services::image_processing_service::classification_output_path(path);
    let Some(data) = std::fs::read(sidecar)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Sidecar>(&bytes).ok())
    else {
        return (Vec::new(), Vec::new());
    };
    let mut categories = Vec::<String>::new();
    for prediction in data
        .classification
        .primary
        .into_iter()
        // Visual is a routing label; its secondary prediction is the category.
        .filter(|prediction| !prediction.label.trim().eq_ignore_ascii_case("visual"))
        .chain(data.classification.secondary)
    {
        let label = prediction.label.trim();
        if !label.is_empty()
            && !categories
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(label))
        {
            categories.push(label.to_owned());
        }
    }
    let tags = data
        .search_keywords
        .into_iter()
        .flat_map(|tag| {
            tag.split(|character: char| character.is_whitespace() || character == '_' || character == '-')
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|tag| !tag.is_empty() && !["some", "different", "item", "group"].iter().any(|word| tag.eq_ignore_ascii_case(word)))
        .fold(Vec::<String>::new(), |mut tags, tag| {
            if !tags
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&tag))
            {
                tags.push(tag);
            }
            tags
        });
    (categories, tags)
}

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
            total = next_total;
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
        counts: None,
        total_count: verified.then_some(total),
        issues,
    }
}

pub(crate) fn validate_drive_counts(
    saved: &DriveMetadata,
    data: &DriveMetadata,
) -> Option<&'static str> {
    if saved.file_count != data.file_count {
        return Some("File counts in SQLite and drive metadata do not match. Data may be out of sync or modified; no counts have been changed.");
    }
    if saved.file_count < 0 {
        return Some("The drive's total file count cannot be negative.");
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

/// Read membership for one known sidecar; never enumerate the drive.
pub(crate) fn sidecar_collection_ids(path: &Path) -> crate::error::AppResult<Vec<String>> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(crate::error::AppError::serialization)?;
    value.get("collectionIds").map(|ids| serde_json::from_value(ids.clone()).map_err(crate::error::AppError::serialization)).unwrap_or_else(|| Ok(Vec::new()))
}

pub(crate) fn add_sidecar_collections(file: &Path, ids: &[String]) -> crate::error::AppResult<()> {
    let path = super::image_processing_service::classification_output_path(file);
    let mut existing = sidecar_collection_ids(&path)?;
    let before = existing.len();
    for id in ids { if !existing.contains(id) { existing.push(id.clone()); } }
    if existing.len() == before { return Ok(()); }
    let bytes = std::fs::read(&path)?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).map_err(crate::error::AppError::serialization)?;
    let object = value.as_object_mut().ok_or_else(|| crate::error::AppError::validation("File metadata must be a JSON object."))?;
    object.insert("collectionIds".into(), serde_json::to_value(existing).map_err(crate::error::AppError::serialization)?);
    crate::system::filesystem::write_file(path, serde_json::to_vec_pretty(&value).map_err(crate::error::AppError::serialization)?)?;
    Ok(())
}
