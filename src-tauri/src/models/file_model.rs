use crate::types::file_type::FileType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
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
