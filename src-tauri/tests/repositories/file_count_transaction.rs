use crate::error::AppError;
use crate::models::storage_model::DriveMetadata;
use crate::repositories::storage_repository::SqliteStorageRepository;

#[tokio::test]
async fn metadata_failure_rolls_back_drive_totals() {
    let root = std::env::temp_dir().join(format!(
        "nexfile-count-transaction-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let database = crate::SqliteDatabase::open(root.join("test.sqlite3"))
        .await
        .unwrap();
    let repository = SqliteStorageRepository::new(database.clone());
    let mut drive = repository
        .insert(&DriveMetadata {
            drive_id: "test".into(),
            drive_name: "Test".into(),
            partition_name: "Test".into(),
            file_count: 1,
            app_used_bytes: 10,
            app_limit_bytes: None,
            priority: 1,
            is_mounted: true,
            created_at_ms: 0,
            updated_at_ms: 0,
        })
        .await
        .unwrap();
    let original = drive.clone();
    drive.file_count = 2;
    drive.app_used_bytes = 20;
    assert!(repository
        .update(&drive, |updated| {
            assert_eq!(updated.file_count, 2);
            Err(AppError::validation("metadata write failed"))
        })
        .await
        .is_err());
    assert_eq!(repository.get("test").await.unwrap(), Some(original));
    for _ in 0..2 {
        repository.update(&drive, |_| Ok(())).await.unwrap();
    }
    let loaded = repository.get("test").await.unwrap().unwrap();
    assert_eq!(loaded.file_count, 2);
    assert_eq!(loaded.app_used_bytes, 20);
    assert_eq!(repository.list().await.unwrap().len(), 1);
    repository.close().await;
    std::fs::remove_dir_all(root).unwrap();
}
