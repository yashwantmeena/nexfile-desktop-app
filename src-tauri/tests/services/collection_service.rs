use crate::models::storage_model::{DriveInfo, DriveMetadata};
use crate::services::{
    collection_service::*,
    file_service::sidecar_collection_ids,
    storage_service::{drive_storage_root, write_drive_metadata},
};

#[test]
fn collection_persistence_does_not_read_file_metadata() {
    let root = std::env::temp_dir().join(format!("collections-crud-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("files")).unwrap();
    std::fs::write(root.join("files/unrelated.jpg.json"), b"invalid JSON").unwrap();
    let mut data = read(&root, "drive").unwrap();
    let id = data.create(" Travel ").unwrap();
    assert_eq!(id.len(), 14);
    let created = data.collections[0].created_at_ms;
    assert!(created > 0);
    assert!(data.create("travel").is_err());

    assert_eq!(data.collections[0].created_at_ms, created);
    save(&root, &data).unwrap();
    let data = read(&root, "drive").unwrap();
    assert_eq!(data.collections[0].name, "Travel");
    assert_eq!(data.collections[0].id, id);
    assert_eq!(
        std::fs::read(root.join("files/unrelated.jpg.json")).unwrap(),
        b"invalid JSON"
    );
    assert!(read(&root, "wrong-drive").is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn import_updates_only_known_sidecar_and_reuses_collection() {
    let root = std::env::temp_dir().join(format!("collections-import-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("files")).unwrap();
    let file = root.join("files/abc.jpg");
    let sidecar = root.join("files/abc.jpg.json");
    std::fs::write(&file, b"photo").unwrap();
    std::fs::write(&sidecar, br#"{"version":1,"name":"Photo.jpg","custom":42}"#).unwrap();
    std::fs::write(root.join("files/other.jpg.json"), b"invalid JSON").unwrap();
    add_file_to_collections(&root, "drive", &file, &[]).unwrap();
    assert!(!root.join("collections.json").exists());
    add_file_to_collections(&root, "drive", &file, &["Travel".into()]).unwrap();
    let data = read(&root, "drive").unwrap();
    let id = data.collections[0].id.clone();
    let saved = std::fs::read(root.join("collections.json")).unwrap();
    add_file_to_collections(&root, "drive", &file, &["travel".into()]).unwrap();
    assert_eq!(std::fs::read(root.join("collections.json")).unwrap(), saved);
    assert_eq!(sidecar_collection_ids(&sidecar).unwrap(), vec![id]);
    let json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&sidecar).unwrap()).unwrap();
    assert_eq!(json["custom"], 42);
    assert_eq!(json["name"], "Photo.jpg");
    assert_eq!(
        std::fs::read(root.join("files/other.jpg.json")).unwrap(),
        b"invalid JSON"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolves_one_name_to_each_drives_local_collection_id() {
    let root = std::env::temp_dir().join(format!("collections-resolve-{}", uuid::Uuid::new_v4()));
    let system_metadata_root = root.join("system");
    let make_drive = |drive_id: &str| {
        let mount_point = root.join(drive_id);
        let info = DriveInfo {
            device_id: format!("device-{drive_id}"),
            drive_name: drive_id.into(),
            partition_name: drive_id.into(),
            file_system: "test".into(),
            total_bytes: 100,
            system_used_bytes: 0,
            is_system: false,
            mount_point,
        };
        let metadata = DriveMetadata {
            drive_id: drive_id.into(),
            drive_name: drive_id.into(),
            partition_name: drive_id.into(),
            app_limit_bytes: Some(100),
            file_count: 0,
            app_used_bytes: 0,
            priority: 1,
            is_mounted: true,
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        (info, metadata)
    };
    let (first_drive, first_saved) = make_drive("drive-a");
    let (second_drive, second_saved) = make_drive("drive-b");
    let mut expected = Vec::new();
    for (drive, saved) in [(&first_drive, &first_saved), (&second_drive, &second_saved)] {
        let storage_root = drive_storage_root(drive, &system_metadata_root);
        std::fs::create_dir_all(&storage_root).unwrap();
        write_drive_metadata(drive, &system_metadata_root, saved).unwrap();
        let mut metadata = read(&storage_root, &saved.drive_id).unwrap();
        let id = metadata.create("Delhi").unwrap();
        save(&storage_root, &metadata).unwrap();
        expected.push((saved.drive_id.clone(), id));
    }
    let saved = vec![first_saved, second_saved];
    let connected = vec![first_drive, second_drive];
    let mut actual = ids_by_name(&saved, &connected, &system_metadata_root, " delhi ").unwrap();
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
    std::fs::remove_dir_all(root).unwrap();
}
