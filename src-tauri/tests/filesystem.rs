use nexfile_desktop_app_lib::{detect_storage_devices, map_disk_kind, StorageKind};
use sysinfo::DiskKind;

#[test]
fn maps_all_disk_kinds() {
    assert_eq!(map_disk_kind(DiskKind::SSD), StorageKind::Ssd);
    assert_eq!(map_disk_kind(DiskKind::HDD), StorageKind::Hdd);
    assert_eq!(map_disk_kind(DiskKind::Unknown(42)), StorageKind::Unknown);
}

#[test]
fn detected_devices_have_valid_capacity_values() {
    for device in detect_storage_devices() {
        assert!(!device.volume_id.is_empty());
        assert!(!device.mount_point.is_empty());
        assert!(device.available_bytes <= device.total_bytes);
        assert_eq!(
            device.used_bytes,
            device.total_bytes - device.available_bytes
        );
    }
}
