use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::models::image_processing_model::ImageProcessingOutput;
use crate::models::indexing_model::IndexDocument;
use crate::models::storage_model::DriveMetadata;
use crate::repositories::indexing_repository::TantivyIndexingRepository;
use crate::utils::constants::{DRIVE_METADATA_FILE, IMPORTED_FILES_DIRECTORY};

#[derive(Clone)]
pub struct IndexingService {
    repository: TantivyIndexingRepository,
}

impl IndexingService {
    pub fn new(repository: TantivyIndexingRepository) -> Self {
        Self { repository }
    }

    pub async fn process_path(&self, path: PathBuf) -> AppResult<()> {
        let service = self.clone();
        tauri::async_runtime::spawn_blocking(move || service.process_blocking(&path))
            .await
            .map_err(AppError::internal)?
    }

    /// Reindexes one bounded queue batch. A failed entry retries the batch, and upserts are
    /// idempotent, so already-completed entries remain safe.
    pub async fn process_batch(&self, paths: Vec<PathBuf>) -> AppResult<()> {
        let service = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let documents = paths
                .iter()
                .map(|path| service.document_for_path(path))
                .collect::<AppResult<Vec<_>>>()?;
            service.repository.upsert_batch(documents)
        })
        .await
        .map_err(AppError::internal)?
    }

    pub async fn delete_batch(&self, drive_id: String, file_ids: Vec<String>) -> AppResult<()> {
        let repository = self.repository.clone();
        tauri::async_runtime::spawn_blocking(move || repository.delete_files(&drive_id, &file_ids))
            .await
            .map_err(AppError::internal)?
    }

    fn process_blocking(&self, path: &Path) -> AppResult<()> {
        self.repository.upsert(self.document_for_path(path)?)
    }

    fn document_for_path(&self, path: &Path) -> AppResult<IndexDocument> {
        let source = IndexSource::read(path)?;
        let Some(output) = source.output else {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(AppError::system_time)
                .and_then(|duration| u64::try_from(duration.as_millis()).map_err(AppError::internal))?;
            return Ok(IndexDocument {
                name: source.name,
                file_id: source.file_id,
                drive_id: source.drive_id,
                created_at_ms: now,
                updated_at_ms: now,
                media_type: None,
                size_bytes: source.size_bytes,
                latitude: None,
                longitude: None,
                object_labels: Vec::new(),
                search_keywords: Vec::new(),
                secondary_labels: Vec::new(),
                categories: Vec::new(),
                collection_ids: source.collection_ids,
                favorite: source.favorite,
            });
        };
        let media_type = normalized_value(output.metadata.media_type.as_deref());
        let location = output.metadata.location.filter(valid_location);

        let mut object_labels = HashSet::new();
        object_labels.extend(
            output
                .object_detection
                .iter()
                .flat_map(|output| output.detections.iter())
                .filter_map(|detection| normalized_value(Some(&detection.label))),
        );

        let mut search_keywords = HashSet::new();
        search_keywords.extend(
            output
                .search_keywords
                .iter()
                .filter(|keyword| !crate::utils::search_tags::is_blocked_search_tag(keyword))
                .filter_map(|keyword| normalized_value(Some(keyword))),
        );

        let mut secondary_labels = HashSet::new();
        let mut categories = HashSet::new();
        for prediction in &output.classification.secondary {
            if let Some(label) = normalized_value(Some(&prediction.label)) {
                secondary_labels.insert(label.clone());
                categories.insert(label);
            }
        }
        for prediction in &output.classification.primary {
            if let Some(label) = normalized_value(Some(&prediction.label)) {
                categories.insert(label);
            }
        }

        Ok(IndexDocument {
            name: output.name,
            file_id: source.file_id,
            drive_id: source.drive_id,
            created_at_ms: output.created_at_ms,
            updated_at_ms: output.updated_at_ms,
            media_type,
            size_bytes: output.metadata.size_bytes,
            latitude: location.as_ref().map(|location| location.latitude),
            longitude: location.as_ref().map(|location| location.longitude),
            object_labels: object_labels.into_iter().collect(),
            search_keywords: search_keywords.into_iter().collect(),
            secondary_labels: secondary_labels.into_iter().collect(),
            categories: categories.into_iter().collect(),
            collection_ids: source.collection_ids,
            favorite: output.favorite,
        })
    }
}

struct IndexSource {
    file_id: String,
    drive_id: String,
    name: String,
    favorite: bool,
    size_bytes: u64,
    collection_ids: Vec<String>,
    output: Option<ImageProcessingOutput>,
}

impl IndexSource {
    fn read(sidecar_path: &Path) -> AppResult<Self> {
        if !sidecar_path.is_file() {
            return Err(AppError::validation(
                "The queued indexing sidecar path is no longer a file.",
            ));
        }
        if !sidecar_path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        {
            return Err(AppError::validation(
                "The queued indexing path is not a JSON sidecar.",
            ));
        }

        let image_path = image_path(sidecar_path)?;
        let metadata = std::fs::metadata(&image_path)?;
        if !metadata.is_file() {
            return Err(AppError::validation(
                "The image associated with the indexing sidecar no longer exists.",
            ));
        }

        let files_directory = sidecar_path
            .parent()
            .ok_or_else(|| AppError::validation("The indexing sidecar has no parent directory."))?;
        if files_directory.file_name().and_then(|name| name.to_str())
            != Some(IMPORTED_FILES_DIRECTORY)
        {
            return Err(AppError::validation(
                "The indexing sidecar is outside the managed files directory.",
            ));
        }
        let storage_root = files_directory.parent().ok_or_else(|| {
            AppError::validation("The managed files directory has no storage root.")
        })?;
        let drive_metadata = serde_json::from_slice::<DriveMetadata>(&std::fs::read(
            storage_root.join(DRIVE_METADATA_FILE),
        )?)
        .map_err(AppError::serialization)?;
        let sidecar = std::fs::read(sidecar_path)?;
        let managed = crate::services::file_service::read_managed_file_metadata(sidecar_path)?;
        let output = serde_json::from_slice::<ImageProcessingOutput>(&sidecar).ok();

        let file_id = required_text(image_path.file_stem(), "The managed image has no file ID.")?;
        let drive_id = drive_metadata.drive_id;
        Ok(Self {
            file_id,
            drive_id,
            name: managed.name,
            favorite: managed.favorite,
            size_bytes: metadata.len(),
            collection_ids: managed.collection_ids,
            output,
        })
    }
}

fn image_path(sidecar_path: &Path) -> AppResult<PathBuf> {
    let image_name = sidecar_path
        .file_stem()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| AppError::validation("The indexing sidecar has no image filename."))?;
    Ok(sidecar_path.with_file_name(image_name))
}

fn required_text(value: Option<&std::ffi::OsStr>, message: &str) -> AppResult<String> {
    value
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| AppError::validation(message))
}

fn normalized_value(value: Option<&str>) -> Option<String> {
    let normalized = value?
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    (!normalized.is_empty()).then_some(normalized)
}

fn valid_location(location: &crate::models::image_processing_model::ImageLocation) -> bool {
    location.latitude.is_finite()
        && location.longitude.is_finite()
        && (-90.0..=90.0).contains(&location.latitude)
        && (-180.0..=180.0).contains(&location.longitude)
}
