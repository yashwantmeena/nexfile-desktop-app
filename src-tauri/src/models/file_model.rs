use crate::types::file_type::FileType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedFileMetadata {
    pub version: u32,
    #[serde(alias = "originalName")]
    pub name: String,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub is_trashed: bool,
    #[serde(default)]
    pub is_deleted: bool,
    #[serde(default)]
    pub collection_ids: Vec<String>,
    #[serde(default)]
    pub captured_at_ms: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchedFile {
    pub id: String,
    pub drive_id: String,
    pub name: String,
    pub path: std::path::PathBuf,
    pub file_type: FileType,
    pub size_bytes: u64,
    pub modified_at_ms: Option<i64>,
    pub captured_at_ms: Option<i64>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub collection_names: Vec<String>,
    pub favorite: bool,
    pub is_trashed: bool,
    pub is_deleted: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePage {
    pub files: Vec<FetchedFile>,
    pub total_count: usize,
    pub next_offset: Option<usize>,
    pub issues: Vec<FileCountIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileTypeCount {
    pub file_type: FileType,
    pub count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCountSummary {
    pub counts: Option<Vec<FileTypeCount>>,
    pub total_count: Option<i64>,
    pub issues: Vec<FileCountIssue>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCountIssue {
    pub drive_id: String,
    pub drive_name: String,
    pub message: String,
}

