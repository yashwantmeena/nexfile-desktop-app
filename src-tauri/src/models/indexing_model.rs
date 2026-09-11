use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase")]
pub enum IndexingJob {
    CreateIndexBatch {
        process_id: String,
        paths: Vec<PathBuf>,
    },
    DeleteIndexBatch {
        process_id: String,
        drive_id: String,
        file_ids: Vec<String>,
    },
}

impl IndexingJob {
    pub fn process_id(&self) -> &str {
        match self {
            Self::CreateIndexBatch { process_id, .. }
            | Self::DeleteIndexBatch { process_id, .. } => {
                process_id
            }
        }
    }

    pub fn subject(&self) -> String {
        match self {
            Self::CreateIndexBatch { paths, .. } => format!("{} files", paths.len()),
            Self::DeleteIndexBatch { drive_id, file_ids, .. } => {
                format!("{drive_id}: {} files", file_ids.len())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexDocument {
    pub name: String,
    pub file_id: String,
    pub drive_id: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub media_type: Option<String>,
    pub size_bytes: u64,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub object_labels: Vec<String>,
    pub search_keywords: Vec<String>,
    pub secondary_labels: Vec<String>,
    pub categories: Vec<String>,
    pub collection_ids: Vec<String>,
    pub favorite: bool,
}
