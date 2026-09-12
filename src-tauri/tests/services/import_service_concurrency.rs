use super::*;
use crate::services::storage_service::StorageService;

#[tokio::test]
async fn stages_copy_while_browsing_holds_metadata_lock() {
    let root = std::env::temp_dir().join(format!("nexfile-copy-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("notes.txt");
    std::fs::write(&source, b"complete contents").unwrap();
    let database = SqliteDatabase::open(root.join("test.sqlite3"))
        .await
        .unwrap();
    let storage = StorageService::new(SqliteStorageRepository::new(database.clone()), root.clone());
    let partition = storage
        .get_storage_data()
        .await
        .unwrap()
        .drives
        .into_iter()
        .find(|drive| drive.is_system)
        .unwrap()
        .partition_name;
    storage.mount_drive(None, &partition).await.unwrap();
    let imports = ImportService::new(
        SqliteBackgroundProcessingRepository::new(database.clone()),
        SqliteStorageRepository::new(database.clone()),
        &database,
        root.clone(),
    )
    .await
    .unwrap();
    let catalog = crate::repositories::collection_repository::save(database.pool(), None, "Travel")
        .await
        .unwrap();
    assert_eq!(catalog[0].id.len(), 14);
    assert!(!root.join("nexfile/collections.json").exists());
    assert!(
        crate::repositories::collection_repository::save(database.pool(), None, "travel")
            .await
            .is_err()
    );
    sqlx::query("INSERT INTO background_processes (process_id, process_type, status, collections) VALUES (?1, 'import_file', 'queued', ?2)").bind("test").bind("[\"Travel\"]").execute(database.pool()).await.unwrap();
    let guard = imports.metadata_lock().lock().await;
    let worker = imports.clone();
    let task = tauri::async_runtime::spawn(async move {
        worker
            .consume(ImportFileJob {
                process_id: "test".into(),
                file_id: "abcdefghijklmn".into(),
                path: source,
            })
            .await
    });
    let directory = root.join("nexfile").join("files");
    let staged = directory.join(".abcdefghijklmn.importing");
    let destination = directory.join("abcdefghijklmn.txt");
    let copied = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if std::fs::read(&staged).ok().as_deref() == Some(b"complete contents") {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    assert!(
        copied.is_ok(),
        "copy must proceed while a browser holds the metadata lock"
    );
    assert!(
        !destination.exists(),
        "staged bytes must not be published yet"
    );
    let page = storage.fetch_files(0, 60, None).await.unwrap();
    assert!(
        page.files.is_empty(),
        "browsing must exclude the staged file"
    );
    drop(guard);
    task.await.unwrap().unwrap();
    assert_eq!(std::fs::read(&destination).unwrap(), b"complete contents");
    assert!(!staged.exists());
    let metadata = super::super::collection_service::read(
        &root.join("nexfile"),
        &storage
            .get_storage_data()
            .await
            .unwrap()
            .drives
            .into_iter()
            .find(|drive| drive.is_system)
            .unwrap()
            .drive_id,
    )
    .unwrap();
    assert_eq!(metadata.collections[0].name, "Travel");
    assert_eq!(
        super::super::file_service::sidecar_collection_ids(&classification_output_path(
            &destination
        ))
        .unwrap(),
        vec![metadata.collections[0].id.clone()]
    );
    assert_ne!(metadata.collections[0].id, catalog[0].id);
    assert_eq!(
        storage.fetch_files(0, 60, None).await.unwrap().files.len(),
        1
    );
    imports.close().await;
    storage.close().await;
    std::fs::remove_dir_all(root).unwrap();
}
