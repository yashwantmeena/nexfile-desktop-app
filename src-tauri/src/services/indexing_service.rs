use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::models::image_processing_model::ImageProcessingOutput;
use crate::models::indexing_model::{IndexDocument, IndexingJob};
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

    pub async fn process(&self, job: IndexingJob) -> AppResult<()> {
        self.process_path(job.path).await
    }

    pub async fn process_path(&self, path: PathBuf) -> AppResult<()> {
        let service = self.clone();
        tauri::async_runtime::spawn_blocking(move || service.process_blocking(&path))
            .await
            .map_err(AppError::internal)?
    }

    fn process_blocking(&self, path: &Path) -> AppResult<()> {
        let source = IndexSource::read(path)?;
        let media_type = normalized_value(source.output.metadata.media_type.as_deref());
        let location = source.output.metadata.location.filter(valid_location);

        let mut object_labels = HashSet::new();
        object_labels.extend(
            source
                .output
                .object_detection
                .iter()
                .flat_map(|output| output.detections.iter())
                .filter_map(|detection| normalized_value(Some(&detection.label))),
        );

        let mut search_keywords = HashSet::new();
        search_keywords.extend(
            source
                .output
                .search_keywords
                .iter()
                .filter(|keyword| !crate::utils::search_tags::is_blocked_search_tag(keyword))
                .filter_map(|keyword| normalized_value(Some(keyword))),
        );

        let mut secondary_labels = HashSet::new();
        let mut categories = HashSet::new();
        for prediction in &source.output.classification.secondary {
            if let Some(label) = normalized_value(Some(&prediction.label)) {
                secondary_labels.insert(label.clone());
                categories.insert(label);
            }
        }
        for prediction in &source.output.classification.primary {
            if let Some(label) = normalized_value(Some(&prediction.label)) {
                categories.insert(label);
            }
        }

        self.repository.upsert(IndexDocument {
            name: source.output.name,
            file_id: source.file_id,
            drive_id: source.drive_id,
            created_at_ms: source.output.created_at_ms,
            updated_at_ms: source.output.updated_at_ms,
            media_type,
            size_bytes: source.output.metadata.size_bytes,
            latitude: location.as_ref().map(|location| location.latitude),
            longitude: location.as_ref().map(|location| location.longitude),
            object_labels: object_labels.into_iter().collect(),
            search_keywords: search_keywords.into_iter().collect(),
            secondary_labels: secondary_labels.into_iter().collect(),
            categories: categories.into_iter().collect(),
            collection_ids: source.collection_ids,
            favorite: source.output.favorite,
        })
    }
}

struct IndexSource {
    file_id: String,
    drive_id: String,
    collection_ids: Vec<String>,
    output: ImageProcessingOutput,
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
        let output = serde_json::from_slice::<ImageProcessingOutput>(&std::fs::read(sidecar_path)?)
            .map_err(AppError::serialization)?;

        let file_id = required_text(image_path.file_stem(), "The managed image has no file ID.")?;
        let drive_id = drive_metadata.drive_id;
        let collection_ids = crate::services::file_service::sidecar_collection_ids(sidecar_path)?;
        Ok(Self {
            file_id,
            drive_id,
            collection_ids,
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
