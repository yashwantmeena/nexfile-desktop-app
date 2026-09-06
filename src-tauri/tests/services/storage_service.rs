use std::time::{SystemTime, UNIX_EPOCH};

use nexfile_desktop_app_lib::{
    DriveConfigurationUpdate, DriveMetadata, SqliteStorageRepository, StorageService,
};

fn test_root(name: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    std::env::temp_dir().join(format!("nexfile-{name}-{}-{unique}", std::process::id()))
}

fn write_drive_metadata(root: &std::path::Path, metadata: &DriveMetadata) {
    write_managed_files(root, metadata);
    let directory = root.join("nexfile");
    std::fs::create_dir_all(&directory).expect("metadata directory should be created");
    std::fs::write(
        directory.join("drive_metadata.json"),
        serde_json::to_vec(metadata).expect("metadata should serialize"),
    )
    .expect("metadata should be written");
}

fn write_managed_files(root: &std::path::Path, metadata: &DriveMetadata) {
    // Mounted-drive reconciliation checks real managed files, not just the fixture JSON.
    let directory = root.join("nexfile/files");
    std::fs::create_dir_all(&directory).unwrap();
    for index in 0..metadata.file_count {
        let file = std::fs::File::create(directory.join(format!("fixture-{index}.jpg"))).unwrap();
        file.set_len(if index == 0 { metadata.app_used_bytes as u64 } else { 0 }).unwrap();
    }
}

#[tokio::test]
async fn merges_database_os_and_drive_metadata() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "nexfile-storage-service-{}-{unique}",
        std::process::id()
    ));
    let metadata_directory = root.join("nexfile");
    let database_path = root.join("test.sqlite3");
    std::fs::create_dir_all(&metadata_directory).expect("metadata directory should be created");

    let metadata = DriveMetadata {
        file_type_counts: nexfile_desktop_app_lib::FileType::ALL
            .into_iter()
            .map(|file_type| nexfile_desktop_app_lib::FileTypeCount {
                file_type,
                count: if file_type == nexfile_desktop_app_lib::FileType::Image {
                    25
                } else {
                    0
                },
            })
            .collect(),
        drive_id: "system-drive".to_owned(),
        drive_name: "Test SSD".to_owned(),
        partition_name: "System (C:)".to_owned(),
        app_limit_bytes: Some(1_000),
        file_count: 25,
        app_used_bytes: 400,
        priority: 0,
        is_mounted: false,
        created_at_ms: 0,
        updated_at_ms: 0,
    };
    std::fs::write(
        metadata_directory.join("drive_metadata.json"),
        serde_json::to_vec(&metadata).expect("metadata should serialize"),
    )
    .expect("metadata should be written");

    write_managed_files(&root, &metadata);
    let data = {
        let repository = SqliteStorageRepository::open(&database_path)
            .await
            .expect("repository should open");
        repository
            .insert(&DriveMetadata {
                file_type_counts: nexfile_desktop_app_lib::FileType::ALL
                    .into_iter()
                    .map(|file_type| nexfile_desktop_app_lib::FileTypeCount {
                        file_type,
                        count: if file_type == nexfile_desktop_app_lib::FileType::Image {
                            metadata.file_count
                        } else {
                            0
                        },
                    })
                    .collect(),
                drive_id: metadata.drive_id.clone(),
                drive_name: metadata.drive_name.clone(),
                partition_name: metadata.partition_name.clone(),
                app_limit_bytes: metadata.app_limit_bytes,
                file_count: metadata.file_count,
                app_used_bytes: metadata.app_used_bytes,
                priority: 1,
                is_mounted: true,
                created_at_ms: 0,
                updated_at_ms: 0,
            })
            .await
            .expect("drive should save");
        repository
            .insert(&DriveMetadata {
                file_type_counts: nexfile_desktop_app_lib::FileType::ALL
                    .into_iter()
                    .map(|file_type| nexfile_desktop_app_lib::FileTypeCount {
                        file_type,
                        count: if file_type == nexfile_desktop_app_lib::FileType::Image {
                            10
                        } else {
                            0
                        },
                    })
                    .collect(),
                drive_id: "missing-drive".to_owned(),
                drive_name: "Disconnected SSD".to_owned(),
                partition_name: "Archive".to_owned(),
                app_limit_bytes: Some(2_000),
                file_count: 10,
                app_used_bytes: 200,
                priority: 2,
                is_mounted: true,
                created_at_ms: 0,
                updated_at_ms: 0,
            })
            .await
            .expect("missing drive should save");

        let service = StorageService::new(repository, root.clone());
        let data = service
            .get_storage_data()
            .await
            .expect("drives should load");
        service.close().await;
        data
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
    // This integration test enumerates real drives; other connected NexFile drives
    // can contribute to the summary in addition to the two fixture drives.
    assert!(data.file_indexed >= 35);
    assert!(data.app_limit_bytes >= 3_000);
    assert!(data.app_used_bytes >= 600);
    assert_eq!(
        data.drives_detected,
        drives.iter().filter(|drive| drive.is_connected).count()
    );

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[tokio::test]
async fn mounts_a_connected_drive_and_creates_its_metadata() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "nexfile-mount-drive-{}-{unique}",
        std::process::id()
    ));
    let database_path = root.join("test.sqlite3");
    std::fs::create_dir_all(&root).expect("test directory should be created");

    let data = {
        let repository = SqliteStorageRepository::open(&database_path)
            .await
            .expect("repository should open");
        let service = StorageService::new(repository, root.clone());
        let initial = service
            .get_storage_data()
            .await
            .expect("connected drives should load");
        let system_drive = initial
            .drives
            .iter()
            .find(|drive| drive.is_system)
            .expect("system drive should be connected");

        assert!(system_drive.drive_id.is_empty());
        assert!(system_drive.device_id.is_none());
        assert!(!system_drive.is_mounted);
        let partition_name = system_drive.partition_name.clone();
        let data = service
            .mount_drive(None, &partition_name)
            .await
            .expect("system drive should mount without an ID");
        service.close().await;
        data
    };

    let mounted = data
        .drives
        .iter()
        .find(|drive| drive.is_system)
        .expect("system drive should remain connected");
    assert!(mounted.is_mounted);
    assert_eq!(mounted.priority, 1);
    assert_eq!(mounted.app_limit_bytes, Some(mounted.total_bytes));

    let metadata_path = root.join("nexfile").join("drive_metadata.json");
    let encoded = std::fs::read(metadata_path).expect("metadata should be written");
    let value = serde_json::from_slice::<serde_json::Value>(&encoded)
        .expect("metadata JSON should deserialize");
    assert!(value.get("deviceId").is_none());
    let metadata =
        serde_json::from_slice::<DriveMetadata>(&encoded).expect("metadata should deserialize");
    assert!(metadata.is_mounted);
    assert_eq!(metadata.drive_id, mounted.drive_id);
    assert_eq!(metadata.app_limit_bytes, Some(mounted.total_bytes));
    uuid::Uuid::parse_str(&metadata.drive_id).expect("new drive ID should be a UUID");

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[tokio::test]
async fn mounts_matching_saved_and_file_metadata_without_changing_usage() {
    let root = test_root("mount-matching");
    let database_path = root.join("test.sqlite3");
    let metadata = DriveMetadata {
        file_type_counts: nexfile_desktop_app_lib::FileType::ALL
            .into_iter()
            .map(|file_type| nexfile_desktop_app_lib::FileTypeCount {
                file_type,
                count: if file_type == nexfile_desktop_app_lib::FileType::Image {
                    25
                } else {
                    0
                },
            })
            .collect(),
        drive_id: "matching-drive".to_owned(),
        drive_name: "Saved SSD".to_owned(),
        partition_name: "Saved partition".to_owned(),
        app_limit_bytes: None,
        file_count: 25,
        app_used_bytes: 400,
        priority: 7,
        is_mounted: false,
        created_at_ms: 0,
        updated_at_ms: 0,
    };
    write_drive_metadata(&root, &metadata);

    let data = {
        let repository = SqliteStorageRepository::open(&database_path)
            .await
            .expect("repository should open");
        repository
            .insert(&metadata)
            .await
            .expect("drive should save");
        let service = StorageService::new(repository, root.clone());
        let partition_name = service
            .get_storage_data()
            .await
            .expect("connected drives should load")
            .drives
            .into_iter()
            .find(|drive| drive.is_system)
            .expect("system drive should be connected")
            .partition_name;
        let data = service
            .mount_drive(None, &partition_name)
            .await
            .expect("matching drive should mount");
        service.close().await;
        data
    };

    let mounted = data
        .drives
        .iter()
        .find(|drive| drive.drive_id == metadata.drive_id)
        .expect("mounted drive should be returned");
    assert!(mounted.is_mounted);
    assert_eq!(mounted.file_count, metadata.file_count);
    assert_eq!(mounted.app_used_bytes, Some(metadata.app_used_bytes));
    assert_eq!(mounted.app_limit_bytes, Some(mounted.total_bytes));
    assert_eq!(mounted.priority, metadata.priority);

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[tokio::test]
async fn saves_file_metadata_without_replacing_a_different_drive_id() {
    let root = test_root("mount-mismatched-id");
    let database_path = root.join("test.sqlite3");
    std::fs::create_dir_all(&root).expect("test directory should be created");

    let detected_partition_name = {
        let repository = SqliteStorageRepository::open(&database_path)
            .await
            .expect("repository should open");
        let service = StorageService::new(repository, root.clone());
        let partition_name = service
            .get_storage_data()
            .await
            .expect("connected drives should load")
            .drives
            .into_iter()
            .find(|drive| drive.is_system)
            .expect("system drive should be connected")
            .partition_name;
        service.close().await;
        partition_name
    };
    let file_metadata = DriveMetadata {
        file_type_counts: nexfile_desktop_app_lib::FileType::ALL
            .into_iter()
            .map(|file_type| nexfile_desktop_app_lib::FileTypeCount {
                file_type,
                count: if file_type == nexfile_desktop_app_lib::FileType::Image {
                    30
                } else {
                    0
                },
            })
            .collect(),
        drive_id: uuid::Uuid::new_v4().to_string(),
        drive_name: "Metadata SSD".to_owned(),
        partition_name: "Metadata partition".to_owned(),
        app_limit_bytes: Some(2_000),
        file_count: 30,
        app_used_bytes: 500,
        priority: 0,
        is_mounted: false,
        created_at_ms: 0,
        updated_at_ms: 0,
    };
    write_drive_metadata(&root, &file_metadata);

    let data = {
        let repository = SqliteStorageRepository::open(&database_path)
            .await
            .expect("repository should reopen");
        repository
            .insert(&DriveMetadata {
                file_type_counts: nexfile_desktop_app_lib::FileType::ALL
                    .into_iter()
                    .map(|file_type| nexfile_desktop_app_lib::FileTypeCount {
                        file_type,
                        count: if file_type == nexfile_desktop_app_lib::FileType::Image {
                            file_metadata.file_count
                        } else {
                            0
                        },
                    })
                    .collect(),
                drive_id: "old-drive-id".to_owned(),
                drive_name: "Old SSD".to_owned(),
                partition_name: file_metadata.partition_name.clone(),
                app_limit_bytes: Some(100),
                file_count: file_metadata.file_count,
                app_used_bytes: file_metadata.app_used_bytes,
                priority: 4,
                is_mounted: false,
                created_at_ms: 0,
                updated_at_ms: 0,
            })
            .await
            .expect("mismatched drive should save");
        let service = StorageService::new(repository, root.clone());
        let data = service
            .mount_drive(None, &detected_partition_name)
            .await
            .expect("metadata drive should mount");
        service.close().await;
        data
    };

    let mounted = data
        .drives
        .iter()
        .find(|drive| drive.drive_id == file_metadata.drive_id)
        .expect("metadata drive should be returned");
    assert!(mounted.is_mounted);
    assert_eq!(mounted.file_count, file_metadata.file_count);
    assert_eq!(mounted.app_used_bytes, Some(file_metadata.app_used_bytes));
    assert_eq!(mounted.app_limit_bytes, file_metadata.app_limit_bytes);

    let repository = SqliteStorageRepository::open(&database_path)
        .await
        .expect("repository should reopen");
    let saved = repository.list().await.expect("saved drives should load");
    assert_eq!(saved.len(), 2);
    assert!(saved
        .iter()
        .any(|saved| saved.drive_id == file_metadata.drive_id));
    assert!(saved.iter().any(|saved| saved.drive_id == "old-drive-id"));
    repository.close().await;

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[tokio::test]
async fn saves_existing_file_metadata_when_the_database_has_no_entry() {
    let root = test_root("mount-file-only");
    let database_path = root.join("test.sqlite3");
    let metadata = DriveMetadata {
        file_type_counts: nexfile_desktop_app_lib::FileType::ALL
            .into_iter()
            .map(|file_type| nexfile_desktop_app_lib::FileTypeCount {
                file_type,
                count: if file_type == nexfile_desktop_app_lib::FileType::Image {
                    40
                } else {
                    0
                },
            })
            .collect(),
        drive_id: uuid::Uuid::new_v4().to_string(),
        drive_name: "Portable SSD".to_owned(),
        partition_name: "Portable partition".to_owned(),
        app_limit_bytes: Some(3_000),
        file_count: 40,
        app_used_bytes: 600,
        priority: 0,
        is_mounted: false,
        created_at_ms: 0,
        updated_at_ms: 0,
    };
    write_drive_metadata(&root, &metadata);

    let data = {
        let repository = SqliteStorageRepository::open(&database_path)
            .await
            .expect("repository should open");
        let service = StorageService::new(repository, root.clone());
        let partition_name = service
            .get_storage_data()
            .await
            .expect("connected drives should load")
            .drives
            .into_iter()
            .find(|drive| drive.is_system)
            .expect("system drive should be connected")
            .partition_name;
        let data = service
            .mount_drive(None, &partition_name)
            .await
            .expect("file metadata should mount");
        service.close().await;
        data
    };

    let mounted = data
        .drives
        .iter()
        .find(|drive| drive.drive_id == metadata.drive_id)
        .expect("metadata drive should be returned");
    assert!(mounted.is_mounted);
    assert_eq!(mounted.file_count, metadata.file_count);
    assert_eq!(mounted.app_used_bytes, Some(metadata.app_used_bytes));

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[tokio::test]
async fn unmounts_a_saved_drive_by_changing_only_its_mounted_flag() {
    let root = test_root("unmount-drive");
    let database_path = root.join("test.sqlite3");
    let metadata = DriveMetadata {
        file_type_counts: nexfile_desktop_app_lib::FileType::ALL
            .into_iter()
            .map(|file_type| nexfile_desktop_app_lib::FileTypeCount {
                file_type,
                count: if file_type == nexfile_desktop_app_lib::FileType::Image {
                    50
                } else {
                    0
                },
            })
            .collect(),
        drive_id: uuid::Uuid::new_v4().to_string(),
        drive_name: "Mounted SSD".to_owned(),
        partition_name: "Mounted partition".to_owned(),
        app_limit_bytes: Some(4_000),
        file_count: 50,
        app_used_bytes: 700,
        priority: 3,
        is_mounted: true,
        created_at_ms: 1,
        updated_at_ms: 1,
    };
    write_drive_metadata(&root, &metadata);

    let data = {
        let repository = SqliteStorageRepository::open(&database_path)
            .await
            .expect("repository should open");
        repository
            .insert(&metadata)
            .await
            .expect("drive should save");
        let service = StorageService::new(repository, root.clone());
        let data = service
            .unmount_drive(&metadata.drive_id)
            .await
            .expect("saved drive should unmount");
        service.close().await;
        data
    };

    let unmounted = data
        .drives
        .iter()
        .find(|drive| drive.drive_id == metadata.drive_id)
        .expect("unmounted drive should be returned");
    assert!(unmounted.is_connected);
    assert!(!unmounted.is_mounted);

    let repository = SqliteStorageRepository::open(&database_path)
        .await
        .expect("repository should reopen");
    let mut expected = metadata;
    expected.is_mounted = false;
    let saved = repository
        .list()
        .await
        .expect("saved drives should load")
        .pop()
        .expect("unmounted drive should remain saved");
    assert!(saved.created_at_ms > 0);
    assert!(saved.updated_at_ms >= saved.created_at_ms);
    expected.created_at_ms = saved.created_at_ms;
    expected.updated_at_ms = saved.updated_at_ms;
    assert_eq!(saved, expected);
    repository.close().await;

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[tokio::test]
async fn updates_mounted_drive_configuration_in_requested_order() {
    let root = test_root("update-priorities");
    let database_path = root.join("test.sqlite3");
    std::fs::create_dir_all(&root).expect("test directory should be created");

    let first = DriveMetadata {
        file_type_counts: nexfile_desktop_app_lib::FileType::ALL
            .into_iter()
            .map(|file_type| nexfile_desktop_app_lib::FileTypeCount {
                file_type,
                count: if file_type == nexfile_desktop_app_lib::FileType::Image {
                    0
                } else {
                    0
                },
            })
            .collect(),
        drive_id: "first-drive".to_owned(),
        drive_name: "First SSD".to_owned(),
        partition_name: "First partition".to_owned(),
        app_limit_bytes: None,
        file_count: 0,
        app_used_bytes: 0,
        priority: 1,
        is_mounted: true,
        created_at_ms: 0,
        updated_at_ms: 0,
    };
    let second = DriveMetadata {
        file_type_counts: nexfile_desktop_app_lib::FileType::ALL
            .into_iter()
            .map(|file_type| nexfile_desktop_app_lib::FileTypeCount {
                file_type,
                count: if file_type == nexfile_desktop_app_lib::FileType::Image {
                    0
                } else {
                    0
                },
            })
            .collect(),
        drive_id: "second-drive".to_owned(),
        drive_name: "Second SSD".to_owned(),
        partition_name: "Second partition".to_owned(),
        app_limit_bytes: None,
        file_count: 0,
        app_used_bytes: 0,
        priority: 2,
        is_mounted: true,
        created_at_ms: 0,
        updated_at_ms: 0,
    };

    {
        let repository = SqliteStorageRepository::open(&database_path)
            .await
            .expect("repository should open");
        repository
            .insert(&first)
            .await
            .expect("first drive should save");
        repository
            .insert(&second)
            .await
            .expect("second drive should save");

        let service = StorageService::new(repository, root.clone());
        service
            .update_drive_configuration(&[
                DriveConfigurationUpdate {
                    drive_id: second.drive_id.clone(),
                    app_limit_bytes: Some(2_000),
                },
                DriveConfigurationUpdate {
                    drive_id: first.drive_id.clone(),
                    app_limit_bytes: Some(1_000),
                },
            ])
            .await
            .expect("configuration should update");
        service.close().await;
    }

    let repository = SqliteStorageRepository::open(&database_path)
        .await
        .expect("repository should reopen");
    let saved = repository.list().await.expect("saved drives should load");
    assert_eq!(
        saved
            .iter()
            .find(|drive| drive.drive_id == first.drive_id)
            .map(|drive| drive.priority),
        Some(2)
    );
    assert_eq!(
        saved
            .iter()
            .find(|drive| drive.drive_id == second.drive_id)
            .map(|drive| drive.priority),
        Some(1)
    );
    assert_eq!(
        saved
            .iter()
            .find(|drive| drive.drive_id == first.drive_id)
            .and_then(|drive| drive.app_limit_bytes),
        Some(1_000)
    );
    assert_eq!(
        saved
            .iter()
            .find(|drive| drive.drive_id == second.drive_id)
            .and_then(|drive| drive.app_limit_bytes),
        Some(2_000)
    );
    repository.close().await;

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[tokio::test]
async fn removes_a_saved_drive_from_the_database_only() {
    let root = test_root("remove-drive");
    let database_path = root.join("test.sqlite3");
    let metadata = DriveMetadata {
        file_type_counts: nexfile_desktop_app_lib::FileType::ALL
            .into_iter()
            .map(|file_type| nexfile_desktop_app_lib::FileTypeCount {
                file_type,
                count: if file_type == nexfile_desktop_app_lib::FileType::Image {
                    60
                } else {
                    0
                },
            })
            .collect(),
        drive_id: uuid::Uuid::new_v4().to_string(),
        drive_name: "Removable SSD".to_owned(),
        partition_name: "Removable partition".to_owned(),
        app_limit_bytes: Some(5_000),
        file_count: 60,
        app_used_bytes: 800,
        priority: 4,
        is_mounted: true,
        created_at_ms: 0,
        updated_at_ms: 0,
    };
    write_drive_metadata(&root, &metadata);

    let data = {
        let repository = SqliteStorageRepository::open(&database_path)
            .await
            .expect("repository should open");
        repository
            .insert(&metadata)
            .await
            .expect("drive should save");
        let service = StorageService::new(repository, root.clone());
        let data = service
            .remove_drive(&metadata.drive_id)
            .await
            .expect("saved drive should be removed");
        service.close().await;
        data
    };

    let detected = data
        .drives
        .iter()
        .find(|drive| drive.drive_id == metadata.drive_id)
        .expect("connected drive metadata should still be detected");
    assert!(detected.is_connected);
    assert!(!detected.is_mounted);

    let repository = SqliteStorageRepository::open(&database_path)
        .await
        .expect("repository should reopen");
    assert!(repository
        .list()
        .await
        .expect("saved drives should load")
        .is_empty());
    repository.close().await;
    assert!(root.join("nexfile").join("drive_metadata.json").exists());

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}
