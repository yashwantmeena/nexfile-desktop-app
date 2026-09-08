use crate::{
    error::{AppError, AppResult},
    models::collection_model::{Collection, CollectionMetadata},
    system::filesystem::write_file,
    utils::{constants::COLLECTION_ID_LENGTH, time::timestamp_ms},
};
use std::path::Path;

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
