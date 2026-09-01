use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use tantivy::indexer::{IndexWriter, IndexWriterOptions};
use tantivy::{TantivyDocument, Term};

use crate::error::{AppError, AppResult};
use crate::mappers::search_mapper::search_tags;
use crate::models::image_processing_model::ImageProcessingOutput;
use crate::models::indexing_model::IndexingJob;
use crate::models::storage_model::DriveMetadata;
use crate::search::{SearchFields, SearchIndex};
use crate::utils::constants::{
    DRIVE_METADATA_FILE, IMPORTED_FILES_DIRECTORY, INDEX_WRITER_MEMORY_BUDGET_BYTES,
};

#[derive(Clone)]
pub struct IndexingService {
    writer: Arc<Mutex<IndexWriter>>,
    fields: SearchFields,
}

impl IndexingService {
    pub fn new(search: &SearchIndex) -> AppResult<Self> {
        let options = IndexWriterOptions::builder()
            .num_worker_threads(1)
            .num_merge_threads(1)
            .memory_budget_per_thread(INDEX_WRITER_MEMORY_BUDGET_BYTES)
            .build();
        let writer = search
            .index()
            .writer_with_options::<TantivyDocument>(options)
            .map_err(AppError::internal)?;
        Ok(Self {
            writer: Arc::new(Mutex::new(writer)),
            fields: search.fields(),
        })
    }

    pub async fn process(&self, job: IndexingJob) -> AppResult<()> {
        let service = self.clone();
        tauri::async_runtime::spawn_blocking(move || service.process_blocking(job))
            .await
            .map_err(AppError::internal)?
    }

    fn process_blocking(&self, job: IndexingJob) -> AppResult<()> {
        let source = IndexSource::read(&job.path)?;
        let mut document = TantivyDocument::default();
        document.add_text(self.fields.file_id, &source.file_id);
        document.add_text(self.fields.drive_id, &source.drive_id);
        document.add_text(self.fields.name, &source.name);
        document.add_text(self.fields.extension, &source.extension);
        for tag in search_tags(&source.output) {
            document.add_text(self.fields.tags, tag);
        }
        if let Some(caption) = source
            .output
            .caption
            .as_deref()
            .filter(|caption| !caption.trim().is_empty())
        {
            document.add_text(self.fields.caption, caption);
        }
        if let Some(ocr) = source
            .output
            .ocr
            .as_ref()
            .map(|ocr| ocr.text.trim())
            .filter(|ocr| !ocr.is_empty())
        {
            document.add_text(self.fields.ocr, ocr);
        }
        document.add_i64(self.fields.modified_at_ms, source.modified_at_ms);
        document.add_u64(self.fields.size_bytes, source.size_bytes);

        let mut writer = self.writer.lock().map_err(|_| {
            AppError::internal(std::io::Error::other(
                "Tantivy index writer lock is poisoned.",
            ))
        })?;
        writer.delete_term(Term::from_field_text(self.fields.file_id, &source.file_id));
        writer.add_document(document).map_err(AppError::internal)?;
        writer.commit().map_err(AppError::internal)?;
        Ok(())
    }
}

struct IndexSource {
    file_id: String,
    drive_id: String,
    name: String,
    extension: String,
    modified_at_ms: i64,
    size_bytes: u64,
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
        let name = required_text(image_path.file_name(), "The managed image has no filename.")?;
        let extension = required_text(
            image_path.extension(),
            "The managed image has no extension.",
        )?
        .to_lowercase();
        let modified_at_ms = i64::try_from(
            metadata
                .modified()?
                .duration_since(UNIX_EPOCH)
                .map_err(AppError::system_time)?
                .as_millis(),
        )
        .map_err(AppError::internal)?;

        Ok(Self {
            file_id,
            drive_id: drive_metadata.drive_id,
            name,
            extension,
            modified_at_ms,
            size_bytes: metadata.len(),
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
