use std::time::{SystemTime, UNIX_EPOCH};

use nexfile_desktop_app_lib::{DriveMetadata, RedbStorageRepository, StorageService};

#[test]
fn merges_database_os_and_drive_metadata() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "nexfile-storage-service-{}-{unique}",
        std::process::id()
    ));
    let metadata_directory = root.join("nexfile");
    let database_path = root.join("test.redb");
    std::fs::create_dir_all(&metadata_directory).expect("metadata directory should be created");

    let metadata = DriveMetadata {
        drive_id: "system-drive".to_owned(),
        drive_name: "Test SSD".to_owned(),
        partition_name: "System (C:)".to_owned(),
        app_limit_bytes: Some(1_000),
        file_count: 25,
        app_used_bytes: 400,
        priority: 0,
        is_mounted: false,
    };
    std::fs::write(
        metadata_directory.join("drive_metadata.json"),
        serde_json::to_vec(&metadata).expect("metadata should serialize"),
    )
    .expect("metadata should be written");

    let data = {
        let repository =
            RedbStorageRepository::open(&database_path).expect("repository should open");
        repository
            .save(&DriveMetadata {
                drive_id: metadata.drive_id.clone(),
                drive_name: metadata.drive_name.clone(),
                partition_name: metadata.partition_name.clone(),
                app_limit_bytes: metadata.app_limit_bytes,
                file_count: metadata.file_count,
                app_used_bytes: metadata.app_used_bytes,
                priority: 1,
                is_mounted: true,
            })
            .expect("drive should save");
        repository
            .save(&DriveMetadata {
                drive_id: "missing-drive".to_owned(),
                drive_name: "Disconnected SSD".to_owned(),
                partition_name: "Archive".to_owned(),
                app_limit_bytes: Some(2_000),
                file_count: 10,
                app_used_bytes: 200,
                priority: 2,
                is_mounted: true,
            })
            .expect("missing drive should save");

        StorageService::new(repository, root.clone())
            .get_storage_data()
            .expect("drives should load")
    };

    let drives = &data.drives;

    let system_drive = drives
        .iter()
        .find(|drive| drive.drive_id == metadata.drive_id)
        .expect("system drive metadata should match");
    assert!(system_drive.is_system);
    assert!(system_drive.is_connected);
    assert!(system_drive.is_mounted);
    assert_eq!(system_drive.app_limit_bytes, metadata.app_limit_bytes);
    assert_eq!(system_drive.app_used_bytes, Some(metadata.app_used_bytes));

    let missing_drive = drives
        .iter()
        .find(|drive| drive.drive_id == "missing-drive")
        .expect("missing database drive should be returned");
    assert!(!missing_drive.is_mounted);
    assert!(!missing_drive.is_connected);
    assert!(missing_drive.system_used_bytes.is_none());
    assert!(missing_drive.available_bytes.is_none());
    assert_eq!(data.file_indexed, 35);
    assert_eq!(data.app_limit_bytes, 3_000);
    assert_eq!(data.app_used_bytes, 600);
    assert_eq!(
        data.drives_detected,
        drives.iter().filter(|drive| drive.is_connected).count()
    );

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}
