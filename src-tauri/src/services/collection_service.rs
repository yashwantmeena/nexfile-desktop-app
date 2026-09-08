use crate::{
    error::{AppError, AppResult},
    models::collection_model::{Collection, CollectionMetadata},
    models::storage_model::{DriveInfo, DriveMetadata},
    system::filesystem::write_file,
    utils::{constants::COLLECTION_ID_LENGTH, time::timestamp_ms},
};
use std::{collections::HashSet, path::Path};

pub(crate) fn read(root: &Path, drive_id: &str) -> AppResult<CollectionMetadata> {
    let bytes = match std::fs::read(root.join("collections.json")) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(CollectionMetadata {
                version: 1,
                drive_id: drive_id.into(),
                collections: Vec::new(),
            })
        }
        Err(error) => return Err(error.into()),
    };
    let metadata: CollectionMetadata =
        serde_json::from_slice(&bytes).map_err(AppError::serialization)?;
    if metadata.version != 1 || metadata.drive_id != drive_id {
        return Err(AppError::validation(
            "Unsupported collection metadata or mismatched drive ID.",
        ));
    }
    Ok(metadata)
}

pub(crate) fn save(root: &Path, metadata: &CollectionMetadata) -> AppResult<()> {
    write_file(
        root.join("collections.json"),
        serde_json::to_vec_pretty(metadata).map_err(AppError::serialization)?,
    )?;
    Ok(())
}

pub(crate) fn ids_by_name(
    saved_drives: &[DriveMetadata],
    connected_drives: &[DriveInfo],
    system_metadata_root: &Path,
    name: &str,
) -> AppResult<Vec<(String, String)>> {
    let name_key = name.trim().to_lowercase();
    if name_key.is_empty() {
        return Ok(Vec::new());
    }
    let mounted_drive_ids = saved_drives
        .iter()
        .filter(|drive| drive.is_mounted)
        .map(|drive| drive.drive_id.as_str())
        .collect::<HashSet<_>>();
    let mut matches = Vec::new();
    for drive in connected_drives {
        let Some(drive_metadata) =
            super::storage_service::read_drive_metadata(drive, system_metadata_root)
        else {
            continue;
        };
        if !mounted_drive_ids.contains(drive_metadata.drive_id.as_str()) {
            continue;
        }
        let storage_root = super::storage_service::drive_storage_root(drive, system_metadata_root);
        let metadata = read(&storage_root, &drive_metadata.drive_id)?;
        for collection in metadata
            .collections
            .iter()
            .filter(|collection| collection.name.trim().to_lowercase() == name_key)
        {
            let key = (drive_metadata.drive_id.clone(), collection.id.clone());
            if !matches.contains(&key) {
                matches.push(key);
            }
        }
    }
    Ok(matches)
}

impl CollectionMetadata {
    pub fn create(&mut self, name: &str) -> AppResult<String> {
        let name = self.valid_name(name)?;
        let now = timestamp_ms()?;
        let id = loop {
            let id = nanoid::nanoid!(COLLECTION_ID_LENGTH);
            if !self.collections.iter().any(|item| item.id == id) {
                break id;
            }
        };
        self.collections.push(Collection {
            id: id.clone(),
            name,
            created_at_ms: now,
            updated_at_ms: now,
        });
        Ok(id)
    }
    fn valid_name(&self, name: &str) -> AppResult<String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 60 || name.chars().any(char::is_control) {
            return Err(AppError::validation(
                "Collection names must contain 1–60 characters without control characters.",
            ));
        }
        if self
            .collections
            .iter()
            .any(|item| item.name.to_lowercase() == name.to_lowercase())
        {
            return Err(AppError::validation(
                "A collection with this name already exists on this drive.",
            ));
        }
        Ok(name.into())
    }
}
