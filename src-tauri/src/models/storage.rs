use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveConfigurationUpdate {
    pub drive_id: String,
    pub app_limit_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveMetadata {
    pub drive_id: String,
    pub drive_name: String,
    pub partition_name: String,
    pub app_limit_bytes: Option<u64>,
    pub file_count: u64,
    pub app_used_bytes: u64,
    #[serde(default)]
    pub priority: u32,
    #[serde(default)]
    pub is_mounted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    pub device_id: String,
    pub drive_name: String,
    pub partition_name: String,
    pub file_system: String,
    pub total_bytes: u64,
    pub system_used_bytes: u64,
    pub is_system: bool,
    #[serde(skip)]
    pub mount_point: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDrive {
    pub drive_id: String,
    pub device_id: Option<String>,
    pub drive_name: String,
    pub partition_name: String,
    pub file_system: String,
    pub total_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_used_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_used_percent: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_used_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_used_percent: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_limit_bytes: Option<u64>,
    pub file_count: u64,
    pub priority: u32,
    pub is_mounted: bool,
    pub is_connected: bool,
    pub is_system: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageData {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub drives_detected: usize,
    pub file_indexed: u64,
    pub app_limit_bytes: u64,
    pub app_used_bytes: u64,
    pub drives: Vec<StorageDrive>,
}
