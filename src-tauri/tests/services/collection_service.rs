use crate::services::{
    collection_service::*, file_service::sidecar_collection_ids,
    import_service::assign_import_collections,
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
    assign_import_collections(&root, "drive", &file, &[]).unwrap();
    assert!(!root.join("collections.json").exists());
    assign_import_collections(&root, "drive", &file, &["Travel".into()]).unwrap();
    let data = read(&root, "drive").unwrap();
    let id = data.collections[0].id.clone();
    let saved = std::fs::read(root.join("collections.json")).unwrap();
    assign_import_collections(&root, "drive", &file, &["travel".into()]).unwrap();
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
