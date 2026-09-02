use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexingJob {
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexDocument {
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
}
