use crate::error::AppError;
use crate::models::file_model::FileTypeCount;
use crate::models::storage_model::DriveMetadata;
use crate::repositories::storage_repository::SqliteStorageRepository;
use crate::types::file_type::FileType;

#[tokio::test]
async fn metadata_failure_rolls_back_drive_and_category_counts() {
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
            file_type_counts: crate::FileType::ALL
                .into_iter()
                .map(|file_type| crate::FileTypeCount {
                    file_type,
                    count: 0,
                })
                .collect(),
            drive_id: "test".into(),
            drive_name: "Test".into(),
            partition_name: "Test".into(),
            file_count: 0,
            app_used_bytes: 0,
            app_limit_bytes: None,
            priority: 1,
            is_mounted: true,
            created_at_ms: 0,
            updated_at_ms: 0,
        })
        .await
        .unwrap();
    let counts = |count| {
        vec![FileTypeCount {
            file_type: FileType::Image,
            count,
        }]
    };
    drive.file_count = 1;
    // Use known timestamps to test updates without relying on clock delays.
    sqlx::query("UPDATE drive_file_type_counts SET created_at_ms = 1, updated_at_ms = 2")
        .execute(database.pool())
        .await
        .unwrap();
    drive.app_used_bytes = 10;
    drive.file_type_counts = counts(1);
    let first = repository.update(&drive, |_| Ok(())).await.unwrap();
    let mut invalid = first.clone();
    invalid.file_count += 1;
    let error = repository
        .update(&invalid, |_| {
            panic!("invalid counts must be rejected before writing metadata");
        })
        .await
        .unwrap_err();
    assert_eq!(error.code(), "VALIDATION_ERROR");
    assert_eq!(repository.get("test").await.unwrap(), Some(first.clone()));
    let mut second_drive = first.clone();
    second_drive.drive_id = "second".into();
    second_drive.file_count = 0;
    second_drive.file_type_counts.clear();
    repository.insert(&second_drive).await.unwrap();
    // A LEFT JOIN must retain drives with no category rows.
    sqlx::query("DELETE FROM drive_file_type_counts WHERE drive_id = 'second'")
        .execute(database.pool())
        .await
        .unwrap();
    let all = repository.list().await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].drive_id, "second");
    assert_eq!(all[0].file_type_counts.len(), 6);
    assert!(all[0].file_type_counts.iter().all(|entry| entry.count == 0));
    assert_eq!(all[1], first);
    assert_eq!(repository.get("test").await.unwrap(), Some(first.clone()));
    assert!(repository.get("missing").await.unwrap().is_none());
    repository.delete("second").await.unwrap();
    let first_timestamps = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT file_type, created_at_ms, updated_at_ms FROM drive_file_type_counts ORDER BY file_type",
    )
    .fetch_all(database.pool())
    .await
    .unwrap();
    for (file_type, created, updated) in &first_timestamps {
        assert_eq!(*created, 1);
        if file_type == "image" {
            assert!(*updated > 2);
        } else {
            assert_eq!(*updated, 2);
        }
    }
    drive.file_count = 2;
    drive.app_used_bytes = 20;
    drive.file_type_counts = counts(2);
    let error = repository
        .update(&drive, |updated| {
            assert_eq!(updated.file_count, 2);
            assert_eq!(
                updated
                    .file_type_counts
                    .iter()
                    .find(|entry| entry.file_type == FileType::Image)
                    .unwrap()
                    .count,
                2
            );
            Err(AppError::validation("metadata write failed"))
        })
        .await
        .unwrap_err();
    assert_eq!(error.code(), "VALIDATION_ERROR");
    let after_rollback = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT file_type, created_at_ms, updated_at_ms FROM drive_file_type_counts ORDER BY file_type",
    )
    .fetch_all(database.pool())
    .await
    .unwrap();
    assert_eq!(after_rollback, first_timestamps);
    assert_eq!(repository.get("test").await.unwrap().unwrap(), first);
    assert_eq!(
        repository
            .get("test")
            .await
            .unwrap()
            .unwrap()
            .file_type_counts[0]
            .count,
        1
    );
    for _ in 0..2 {
        repository.update(&drive, |_| Ok(())).await.unwrap();
    }
    assert_eq!(repository.get("test").await.unwrap().unwrap().file_count, 2);
    assert_eq!(
        repository
            .get("test")
            .await
            .unwrap()
            .unwrap()
            .file_type_counts[0]
            .count,
        2
    );
    repository.close().await;
    std::fs::remove_dir_all(root).unwrap();
}
