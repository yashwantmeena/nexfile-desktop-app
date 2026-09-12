use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::types::file_type::FileType;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkOperationJob {
    pub process_id: String,
    pub expected_count: u64,
    pub operation: BulkOperation,
    pub filters: BulkOperationFilters,
    /// Files with a filesystem modified time after this value are not part of this task.
    pub snapshot_at_ms: i64,
    /// When present, only these IDs are dispatched. When absent, the filters define membership.
    #[serde(default)]
    pub selected_ids: Option<Vec<String>>,
}

impl BulkOperationJob {
    pub fn subject(&self) -> String {
        format!("{} snapshot_at_ms={}", self.operation.as_str(), self.snapshot_at_ms)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BulkOperation {
    MoveToTrash,
    SetFavorite { favorite: bool },
}

impl BulkOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MoveToTrash => "move_to_trash",
            Self::SetFavorite { .. } => "set_favorite",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkOperationFilters {
    pub query: String,
    pub search_mode: String,
    pub tags: Vec<String>,
    pub collection: Option<String>,
    pub media_type: Option<FileType>,
    pub favorite_only: bool,
    pub trash_only: bool,
    pub model_category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkFileTarget {
    pub id: String,
    pub drive_id: String,
    pub path: PathBuf,
}
