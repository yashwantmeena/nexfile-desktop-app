use serde::{Deserialize, Serialize};

use crate::types::background_process_status::BackgroundProcessStatus;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundProcess {
    pub process_id: String,
    pub process_type: String,
    pub status: BackgroundProcessStatus,
    pub priority: u32,
    pub total_items: u64,
    pub processed_items: u64,
    pub failed_items: u64,
    pub remark: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: Option<u64>,
}
