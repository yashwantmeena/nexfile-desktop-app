use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::delete_model::DeleteFileJob;
use super::import_model::ImportFileJob;

/// Jobs handled by the shared file-processing queue.
///
/// Keeping the operation in one tagged enum allows import, export, and delete
/// jobs to share one durable Apalis queue without competing deserializers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", content = "job", rename_all = "camelCase")]
pub enum FileProcessingJob {
    Import(ImportFileJob),
    Export(ExportFileJob),
    Delete(DeleteFileJob),
    UpdateMetadata(UpdateFileMetadataJob),
}

impl FileProcessingJob {
    pub fn process_id(&self) -> &str {
        match self {
            Self::Import(job) => &job.process_id,
            Self::Export(job) => &job.process_id,
            Self::Delete(job) => &job.process_id,
            Self::UpdateMetadata(job) => &job.process_id,
        }
    }

    pub fn operation(&self) -> &'static str {
        match self {
            Self::Import(_) => "import",
            Self::Export(_) => "export",
            Self::Delete(_) => "delete",
            Self::UpdateMetadata(_) => "update_metadata",
        }
    }

    pub fn subject(&self) -> String {
        match self {
            Self::Import(job) => job.path.display().to_string(),
            Self::Export(job) => job.destination.display().to_string(),
            Self::Delete(job) => job.path.display().to_string(),
            Self::UpdateMetadata(job) => job.path.display().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFileJob {
    pub process_id: String,
    pub source: PathBuf,
    pub destination: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFileMetadataJob {
    pub process_id: String,
    pub drive_id: String,
    pub path: PathBuf,
    pub favorite: Option<bool>,
    pub is_trashed: Option<bool>,
}
