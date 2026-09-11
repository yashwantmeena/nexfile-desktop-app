use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFileJob {
    pub process_id: String,
    pub file_id: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub file_count: u64,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub mounted_drive_count: u64,
    pub can_import_all: bool,
}
