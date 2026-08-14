use nexfile_desktop_app_lib::{
    choose_storage_target, StorageAllocation, StorageDevice, StorageKind,
};

fn allocation(volume_id: &str, priority: u32, quota: u64, used: u64) -> StorageAllocation {
    StorageAllocation {
        schema_version: 1,
        priority,
        volume_id: volume_id.into(),
        physical_disk_id: Some(format!("PHYSICALDRIVE{}", priority - 1)),
        hardware_model: Some(format!("Test SSD {priority}")),
        volume_label: None,
        mount_point: format!("{}:\\", char::from(b'C' + priority as u8)),
        quota_bytes: quota,
        vault_used_bytes: used,
        updated_at_unix_ms: 1_700_000_000_000,
    }
}

fn device(volume_id: &str, available: u64) -> StorageDevice {
    StorageDevice {
        volume_id: volume_id.into(),
        name: "Test Disk".into(),
        mount_point: "D:\\".into(),
        file_system: "NTFS".into(),
        physical_disk_id: Some("PHYSICALDRIVE0".into()),
        hardware_model: Some("Test SSD".into()),
        volume_label: None,
        kind: StorageKind::Ssd,
        total_bytes: 2_000,
        available_bytes: available,
        used_bytes: 2_000 - available,
        is_removable: false,
        is_read_only: false,
    }
}

#[test]
fn selects_next_priority_when_first_volume_is_full() {
    let allocations = vec![
        allocation("volume-a", 1, 500, 500),
        allocation("volume-b", 2, 1_000, 200),
    ];
    let devices = vec![device("volume-a", 500), device("volume-b", 900)];

    let target = choose_storage_target(&allocations, &devices, 500)
        .expect("second volume should have enough room");

    assert_eq!(target.volume_id, "volume-b");
    assert_eq!(target.mount_point, "D:\\");
    assert_eq!(target.priority, 2);
    assert_eq!(target.writable_bytes, 800);
}
