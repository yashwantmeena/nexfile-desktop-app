use std::path::Path;

use crate::models::file_model::{FileCountIssue, FileCountSummary};
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
    fetch_matching_files_with_trash(
        snapshots,
        connected,
        system_metadata_root,
        media_type,
        offset,
        limit,
        None,
        false,
    )
}

#[allow(dead_code)] // Retained as the default active-files helper for integration tests.
pub(crate) fn fetch_matching_files(
    snapshots: Vec<DriveMetadata>,
    connected: Vec<DriveInfo>,
    system_metadata_root: &Path,
    media_type: Option<crate::types::file_type::FileType>,
    offset: usize,
    limit: usize,
    indexed_matches: Option<&std::collections::HashSet<(String, String)>>,
) -> crate::models::file_model::FilePage {
    fetch_matching_files_with_trash(
        snapshots,
        connected,
        system_metadata_root,
        media_type,
        offset,
        limit,
        indexed_matches,
        false,
    )
}

pub(crate) fn fetch_matching_files_with_trash(
    snapshots: Vec<DriveMetadata>,
    connected: Vec<DriveInfo>,
    system_metadata_root: &Path,
    media_type: Option<crate::types::file_type::FileType>,
    offset: usize,
    limit: usize,
    indexed_matches: Option<&std::collections::HashSet<(String, String)>>,
    trash_only: bool,
) -> crate::models::file_model::FilePage {
    use crate::models::file_model::{FetchedFile, FilePage};
    use crate::services::storage_service::{drive_storage_root, is_generated_image_sidecar, read_drive_metadata};
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
        let storage_root = drive_storage_root(drive, system_metadata_root);
        let directory = storage_root.join(IMPORTED_FILES_DIRECTORY);
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
            let managed_metadata = read_managed_file_metadata(
                &crate::services::image_processing_service::classification_output_path(&path),
            )
            .ok();
            let name = managed_metadata
                .as_ref()
                .map(|metadata| metadata.name.trim())
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| managed_name.clone());
            let favorite = managed_metadata.as_ref().is_some_and(|metadata| metadata.favorite);
            let is_trashed = managed_metadata.as_ref().is_some_and(|metadata| metadata.is_trashed);
            let is_deleted = managed_metadata.as_ref().is_some_and(|metadata| metadata.is_deleted);
            if (trash_only && (!is_trashed || is_deleted))
                || (!trash_only && (is_trashed || is_deleted))
            {
                continue;
            }
            let collection_names = read_collection_names(&path, &storage_root, &saved.drive_id);
            files.push(FetchedFile {
                id: format!("{}:{}", saved.drive_id, managed_name),
                drive_id: saved.drive_id.clone(),
                name,
                file_type: crate::mappers::file_mapper::file_type_from_path(&path),
                path,
                size_bytes: metadata.len(),
                modified_at_ms,
                captured_at_ms: managed_metadata
                    .as_ref()
                    .and_then(|metadata| metadata.captured_at_ms)
                    .or_else(|| Some(fallback_captured_at_ms(&metadata))),
                categories: Vec::new(),
                tags: Vec::new(),
                collection_names,
                favorite,
                is_trashed,
                is_deleted,
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

pub(crate) fn read_managed_file_metadata(
    path: &Path,
) -> crate::error::AppResult<crate::models::file_model::ManagedFileMetadata> {
    serde_json::from_slice(&std::fs::read(path)?)
        .map_err(crate::error::AppError::serialization)
}

fn fallback_captured_at_ms(metadata: &std::fs::Metadata) -> i64 {
    let time = metadata
        .created()
        .or_else(|_| metadata.modified())
        .unwrap_or_else(|_| std::time::SystemTime::now());
    match time.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_millis()).unwrap_or_default(),
        Err(error) => i64::try_from(error.duration().as_millis())
            .map(|milliseconds| -milliseconds)
            .unwrap_or_default(),
    }
}

pub(crate) fn read_collection_names(file_path: &Path, storage_root: &Path, drive_id: &str) -> Vec<String> {
    let ids = crate::services::file_service::sidecar_collection_ids(
        &crate::services::image_processing_service::classification_output_path(file_path),
    )
    .unwrap_or_default();
    if ids.is_empty() {
        return Vec::new();
    }
    let Ok(metadata) = crate::services::collection_service::read(storage_root, drive_id) else {
        return Vec::new();
    };
    ids.into_iter()
        .filter_map(|id| metadata.collections.iter().find(|item| item.id == id).map(|item| item.name.clone()))
        .collect()
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
        .filter(|tag| !crate::utils::search_tags::is_blocked_search_tag(tag))
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

/// Update any requested user metadata fields while preserving model output.
pub(crate) fn replace_sidecar_metadata(
    sidecar: &Path,
    name: Option<&str>,
    category: Option<&str>,
    tags: Option<&[String]>,
    collection_ids: Option<&[String]>,
    favorite: Option<bool>,
    is_trashed: Option<bool>,
    is_deleted: Option<bool>,
) -> crate::error::AppResult<crate::models::file_model::ManagedFileMetadata> {
    let bytes = std::fs::read(sidecar)?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(crate::error::AppError::serialization)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| crate::error::AppError::validation("File metadata must be a JSON object."))?;
    if let Some(name) = name {
        object.insert("name".into(), serde_json::Value::String(name.to_owned()));
    }
    if let Some(category) = category {
        let secondary = object
            .get_mut("classification")
            .and_then(serde_json::Value::as_object_mut)
            .and_then(|classification| classification.get_mut("secondary"))
            .and_then(serde_json::Value::as_array_mut)
            .ok_or_else(|| {
                crate::error::AppError::validation("Image category metadata is unavailable.")
            })?;
        if category.is_empty() {
            secondary.clear();
        } else {
            if let Some(prediction) =
                secondary.first_mut().and_then(serde_json::Value::as_object_mut)
            {
                prediction.insert(
                    "label".into(),
                    serde_json::Value::String(category.to_owned()),
                );
            } else {
                secondary.push(
                    serde_json::json!({ "label": category, "parentLabel": null, "score": 1.0 }),
                );
            }
        }
    }
    if let Some(tags) = tags {
        object.insert(
            "searchKeywords".into(),
            serde_json::to_value(tags).map_err(crate::error::AppError::serialization)?,
        );
    }
    if let Some(collection_ids) = collection_ids {
        object.insert(
            "collectionIds".into(),
            serde_json::to_value(collection_ids).map_err(crate::error::AppError::serialization)?,
        );
    }
    if let Some(favorite) = favorite {
        object.insert("favorite".into(), serde_json::Value::Bool(favorite));
    }
    if let Some(is_trashed) = is_trashed {
        object.insert("isTrashed".into(), serde_json::Value::Bool(is_trashed));
    }
    if let Some(is_deleted) = is_deleted {
        object.insert("isDeleted".into(), serde_json::Value::Bool(is_deleted));
    }
    let bytes = serde_json::to_vec_pretty(&value).map_err(crate::error::AppError::serialization)?;
    let metadata = serde_json::from_slice(&bytes).map_err(crate::error::AppError::serialization)?;
    crate::system::filesystem::write_file(sidecar, bytes)?;
    Ok(metadata)
}


