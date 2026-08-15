use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    pub drive_id: String,
    pub drive_name: String,
    pub partition_name: String,
    pub file_system: String,
    pub total_capacity: u64,
    pub system_used: u64,
}
