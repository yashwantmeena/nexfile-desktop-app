use serde::{Deserialize, Serialize};

/// The physical storage technology reported by the operating system.
///
/// `Unknown` is expected for storage such as some USB drives, network mounts,
/// virtual disks, and devices whose drivers do not expose the media kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageKind {
    Ssd,
    Hdd,
    Unknown,
}

/// A mounted storage volume available to NexFile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDevice {
    /// Stable volume identifier when the operating system exposes one. Falls
    /// back to the mount point on platforms where no volume ID is available.
    pub volume_id: String,
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub physical_disk_id: Option<String>,
    pub hardware_model: Option<String>,
    pub volume_label: Option<String>,
    pub kind: StorageKind,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub is_removable: bool,
    pub is_read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetStorageAllocationRequest {
    pub volume_id: String,
    pub quota_bytes: u64,
    pub priority: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectStorageTargetRequest {
    pub required_bytes: u64,
}

/// Persistent policy describing where and how much data NexFile may store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageAllocation {
    pub schema_version: u16,
    pub priority: u32,
    pub volume_id: String,
    pub physical_disk_id: Option<String>,
    pub hardware_model: Option<String>,
    pub volume_label: Option<String>,
    pub mount_point: String,
    pub quota_bytes: u64,
    pub vault_used_bytes: u64,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageTarget {
    pub volume_id: String,
    pub physical_disk_id: Option<String>,
    pub hardware_model: Option<String>,
    pub mount_point: String,
    pub priority: u32,
    pub quota_bytes: u64,
    pub vault_used_bytes: u64,
    pub device_available_bytes: u64,
    pub writable_bytes: u64,
}
