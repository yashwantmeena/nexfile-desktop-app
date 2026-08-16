use std::time::{SystemTime, UNIX_EPOCH};

use nexfile_desktop_app_lib::{DriveMetadata, RedbStorageRepository, StorageService};

fn test_root(name: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    std::env::temp_dir().join(format!("nexfile-{name}-{}-{unique}", std::process::id()))
}

fn write_drive_metadata(root: &std::path::Path, metadata: &DriveMetadata) {
    let directory = root.join("nexfile");
    std::fs::create_dir_all(&directory).expect("metadata directory should be created");
    std::fs::write(
        directory.join("drive_metadata.json"),
        serde_json::to_vec(metadata).expect("metadata should serialize"),
    )
    .expect("metadata should be written");
}

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

#[test]
fn mounts_a_connected_drive_and_creates_its_metadata() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "nexfile-mount-drive-{}-{unique}",
        std::process::id()
    ));
    let database_path = root.join("test.redb");
    std::fs::create_dir_all(&root).expect("test directory should be created");

    let data = {
        let repository =
            RedbStorageRepository::open(&database_path).expect("repository should open");
        let service = StorageService::new(repository, root.clone());
        let initial = service
            .get_storage_data()
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
        service
            .mount_drive(None, &partition_name)
            .expect("system drive should mount without an ID")
    };

    let mounted = data
        .drives
        .iter()
        .find(|drive| drive.is_system)
        .expect("system drive should remain connected");
    assert!(mounted.is_mounted);
    assert_eq!(mounted.priority, 1);

    let metadata_path = root.join("nexfile").join("drive_metadata.json");
    let encoded = std::fs::read(metadata_path).expect("metadata should be written");
    let value = serde_json::from_slice::<serde_json::Value>(&encoded)
        .expect("metadata JSON should deserialize");
    assert!(value.get("deviceId").is_none());
    let metadata =
        serde_json::from_slice::<DriveMetadata>(&encoded).expect("metadata should deserialize");
    assert!(metadata.is_mounted);
    assert_eq!(metadata.drive_id, mounted.drive_id);
    uuid::Uuid::parse_str(&metadata.drive_id).expect("new drive ID should be a UUID");

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[test]
fn mounts_matching_saved_and_file_metadata_without_changing_usage() {
    let root = test_root("mount-matching");
    let database_path = root.join("test.redb");
    let metadata = DriveMetadata {
        drive_id: "matching-drive".to_owned(),
        drive_name: "Saved SSD".to_owned(),
        partition_name: "Saved partition".to_owned(),
        app_limit_bytes: Some(1_000),
        file_count: 25,
        app_used_bytes: 400,
        priority: 7,
        is_mounted: false,
    };
    write_drive_metadata(&root, &metadata);

    let data = {
        let repository =
            RedbStorageRepository::open(&database_path).expect("repository should open");
        repository.save(&metadata).expect("drive should save");
        let service = StorageService::new(repository, root.clone());
        let partition_name = service
            .get_storage_data()
            .expect("connected drives should load")
            .drives
            .into_iter()
            .find(|drive| drive.is_system)
            .expect("system drive should be connected")
            .partition_name;
        service
            .mount_drive(None, &partition_name)
            .expect("matching drive should mount")
    };

    let mounted = data
        .drives
        .iter()
        .find(|drive| drive.drive_id == metadata.drive_id)
        .expect("mounted drive should be returned");
    assert!(mounted.is_mounted);
    assert_eq!(mounted.file_count, metadata.file_count);
    assert_eq!(mounted.app_used_bytes, Some(metadata.app_used_bytes));
    assert_eq!(mounted.priority, metadata.priority);

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[test]
fn saves_file_metadata_without_replacing_a_different_drive_id() {
    let root = test_root("mount-mismatched-id");
    let database_path = root.join("test.redb");
    std::fs::create_dir_all(&root).expect("test directory should be created");

    let detected_partition_name = {
        let repository =
            RedbStorageRepository::open(&database_path).expect("repository should open");
        let service = StorageService::new(repository, root.clone());
        service
            .get_storage_data()
            .expect("connected drives should load")
            .drives
            .into_iter()
            .find(|drive| drive.is_system)
            .expect("system drive should be connected")
            .partition_name
    };
    let file_metadata = DriveMetadata {
        drive_id: uuid::Uuid::new_v4().to_string(),
        drive_name: "Metadata SSD".to_owned(),
        partition_name: "Metadata partition".to_owned(),
        app_limit_bytes: Some(2_000),
        file_count: 30,
        app_used_bytes: 500,
        priority: 0,
        is_mounted: false,
    };
    write_drive_metadata(&root, &file_metadata);

    let data = {
        let repository =
            RedbStorageRepository::open(&database_path).expect("repository should reopen");
        repository
            .save(&DriveMetadata {
                drive_id: "old-drive-id".to_owned(),
                drive_name: "Old SSD".to_owned(),
                partition_name: file_metadata.partition_name.clone(),
                app_limit_bytes: Some(100),
                file_count: file_metadata.file_count,
                app_used_bytes: file_metadata.app_used_bytes,
                priority: 4,
                is_mounted: false,
            })
            .expect("mismatched drive should save");
        StorageService::new(repository, root.clone())
            .mount_drive(None, &detected_partition_name)
            .expect("metadata drive should mount")
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

    let repository = RedbStorageRepository::open(&database_path).expect("repository should reopen");
    let saved = repository.list().expect("saved drives should load");
    assert_eq!(saved.len(), 2);
    assert!(saved
        .iter()
        .any(|saved| saved.drive_id == file_metadata.drive_id));
    assert!(saved.iter().any(|saved| saved.drive_id == "old-drive-id"));
    drop(repository);

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[test]
fn saves_existing_file_metadata_when_the_database_has_no_entry() {
    let root = test_root("mount-file-only");
    let database_path = root.join("test.redb");
    let metadata = DriveMetadata {
        drive_id: uuid::Uuid::new_v4().to_string(),
        drive_name: "Portable SSD".to_owned(),
        partition_name: "Portable partition".to_owned(),
        app_limit_bytes: Some(3_000),
        file_count: 40,
        app_used_bytes: 600,
        priority: 0,
        is_mounted: false,
    };
    write_drive_metadata(&root, &metadata);

    let data = {
        let repository =
            RedbStorageRepository::open(&database_path).expect("repository should open");
        let service = StorageService::new(repository, root.clone());
        let partition_name = service
            .get_storage_data()
            .expect("connected drives should load")
            .drives
            .into_iter()
            .find(|drive| drive.is_system)
            .expect("system drive should be connected")
            .partition_name;
        service
            .mount_drive(None, &partition_name)
            .expect("file metadata should mount")
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

#[test]
fn unmounts_a_saved_drive_by_changing_only_its_mounted_flag() {
    let root = test_root("unmount-drive");
    let database_path = root.join("test.redb");
    let metadata = DriveMetadata {
        drive_id: uuid::Uuid::new_v4().to_string(),
        drive_name: "Mounted SSD".to_owned(),
        partition_name: "Mounted partition".to_owned(),
        app_limit_bytes: Some(4_000),
        file_count: 50,
        app_used_bytes: 700,
        priority: 3,
        is_mounted: true,
    };
    write_drive_metadata(&root, &metadata);

    let data = {
        let repository =
            RedbStorageRepository::open(&database_path).expect("repository should open");
        repository.save(&metadata).expect("drive should save");
        StorageService::new(repository, root.clone())
            .unmount_drive(&metadata.drive_id)
            .expect("saved drive should unmount")
    };

    let unmounted = data
        .drives
        .iter()
        .find(|drive| drive.drive_id == metadata.drive_id)
        .expect("unmounted drive should be returned");
    assert!(unmounted.is_connected);
    assert!(!unmounted.is_mounted);

    let repository = RedbStorageRepository::open(&database_path).expect("repository should reopen");
    let mut expected = metadata;
    expected.is_mounted = false;
    assert_eq!(
        repository.list().expect("saved drives should load"),
        vec![expected]
    );
    drop(repository);

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[test]
fn removes_a_saved_drive_from_the_database_only() {
    let root = test_root("remove-drive");
    let database_path = root.join("test.redb");
    let metadata = DriveMetadata {
        drive_id: uuid::Uuid::new_v4().to_string(),
        drive_name: "Removable SSD".to_owned(),
        partition_name: "Removable partition".to_owned(),
        app_limit_bytes: Some(5_000),
        file_count: 60,
        app_used_bytes: 800,
        priority: 4,
        is_mounted: true,
    };
    write_drive_metadata(&root, &metadata);

    let data = {
        let repository =
            RedbStorageRepository::open(&database_path).expect("repository should open");
        repository.save(&metadata).expect("drive should save");
        StorageService::new(repository, root.clone())
            .remove_drive(&metadata.drive_id)
            .expect("saved drive should be removed")
    };

    let detected = data
        .drives
        .iter()
        .find(|drive| drive.drive_id == metadata.drive_id)
        .expect("connected drive metadata should still be detected");
    assert!(detected.is_connected);
    assert!(!detected.is_mounted);

    let repository = RedbStorageRepository::open(&database_path).expect("repository should reopen");
    assert!(repository
        .list()
        .expect("saved drives should load")
        .is_empty());
    drop(repository);
    assert!(root.join("nexfile").join("drive_metadata.json").exists());

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}
