use std::sync::atomic::{AtomicU64, Ordering};

use nexfile_desktop_app_lib::{DriveMetadata, FileType, FileTypeCount, SqliteStorageRepository};

static NEXT_TEST_DATABASE: AtomicU64 = AtomicU64::new(0);

fn test_database_path() -> std::path::PathBuf {
    let unique = NEXT_TEST_DATABASE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "nexfile-drives-{}-{unique}.sqlite3",
        std::process::id()
    ))
}

#[tokio::test]
async fn saves_aggregate_counts_without_per_file_records() {
    let path = test_database_path();
    let repository = SqliteStorageRepository::open(&path)
        .await
        .expect("repository should open");
    let mut drive = repository
        .insert(&DriveMetadata {
            file_type_counts: nexfile_desktop_app_lib::FileType::ALL
                .into_iter()
                .map(|file_type| nexfile_desktop_app_lib::FileTypeCount {
                    file_type,
                    count: 0,
                })
                .collect(),
            drive_id: "c".to_owned(),
            drive_name: "Test SSD".to_owned(),
            partition_name: "System (C:)".to_owned(),
            app_limit_bytes: Some(1_000),
            file_count: 0,
            app_used_bytes: 0,
            priority: 1,
            is_mounted: true,
            created_at_ms: 0,
            updated_at_ms: 0,
        })
        .await
        .expect("drive should save");

    drive.file_count = 1;
    drive.app_used_bytes = 100;
    drive.file_type_counts = vec![FileTypeCount {
        file_type: FileType::Image,
        count: 1,
    }];
    repository
        .update(&drive, |_| Ok(()))
        .await
        .expect("image import should be recorded");
    drive.file_type_counts = vec![FileTypeCount {
        file_type: FileType::Image,
        count: 1,
    }];
    repository
        .update(&drive, |_| Ok(()))
        .await
        .expect("replayed import should be idempotent");

    drive.file_count = 2;
    drive.app_used_bytes = 150;
    drive.file_type_counts = vec![
        FileTypeCount {
            file_type: FileType::Image,
            count: 1,
        },
        FileTypeCount {
            file_type: FileType::Document,
            count: 1,
        },
    ];
    repository
        .update(&drive, |_| Ok(()))
        .await
        .expect("document import should be recorded");

    let counts = repository
        .get("c")
        .await
        .expect("drive should load")
        .expect("drive should exist")
        .file_type_counts;
    assert_eq!(
        counts
            .iter()
            .find(|entry| entry.file_type == FileType::Image)
            .unwrap()
            .count,
        1
    );
    assert_eq!(
        counts
            .iter()
            .find(|entry| entry.file_type == FileType::Document)
            .unwrap()
            .count,
        1
    );
    assert_eq!(
        counts
            .iter()
            .find(|entry| entry.file_type == FileType::Video)
            .unwrap()
            .count,
        0
    );
    assert_eq!(
        counts
            .iter()
            .find(|entry| entry.file_type == FileType::Audio)
            .unwrap()
            .count,
        0
    );
    assert_eq!(
        counts
            .iter()
            .find(|entry| entry.file_type == FileType::Archive)
            .unwrap()
            .count,
        0
    );
    assert_eq!(
        counts
            .iter()
            .find(|entry| entry.file_type == FileType::Other)
            .unwrap()
            .count,
        0
    );

    repository.close().await;
    std::fs::remove_file(path).expect("test database should be removable");
}

#[tokio::test]
async fn saves_and_lists_drives() {
    let path = test_database_path();
    let mut drive = DriveMetadata {
        file_type_counts: nexfile_desktop_app_lib::FileType::ALL
            .into_iter()
            .map(|file_type| nexfile_desktop_app_lib::FileTypeCount {
                file_type,
                count: 0,
            })
            .collect(),
        drive_id: "c".to_owned(),
        drive_name: "Test SSD".to_owned(),
        partition_name: "System (C:)".to_owned(),
        app_limit_bytes: Some(100),
        file_count: 25,
        app_used_bytes: 40,
        priority: 1,
        is_mounted: true,
        created_at_ms: 1,
        updated_at_ms: 2,
    };

    {
        let repository = SqliteStorageRepository::open(&path)
            .await
            .expect("repository should open");
        drive.file_type_counts[0].count = 25;
        let inserted = repository.insert(&drive).await.expect("drive should save");
        assert_eq!(inserted.file_type_counts, drive.file_type_counts);
        assert!(inserted.created_at_ms > 0);
        assert_eq!(inserted.updated_at_ms, inserted.created_at_ms);
        let saved = repository
            .list()
            .await
            .expect("drives should load")
            .pop()
            .expect("saved drive should exist");
        assert_eq!(saved, inserted);

        drive.created_at_ms = inserted.created_at_ms;
        drive.is_mounted = false;
        drive.file_type_counts[0].count = 20;
        drive.file_type_counts[3].count = 5;
        let updated = repository
            .update(&drive, |_| Ok(()))
            .await
            .expect("drive update should save");
        assert_eq!(updated.file_type_counts, drive.file_type_counts);
        assert_eq!(updated.created_at_ms, inserted.created_at_ms);
        assert!(updated.updated_at_ms >= inserted.updated_at_ms);
        let loaded = repository
            .list()
            .await
            .expect("updated drive should load")
            .pop()
            .expect("updated drive should exist");
        assert_eq!(loaded, updated);
        assert_eq!(
            repository.get("c").await.expect("drive should load"),
            Some(updated)
        );

        assert!(repository.delete("c").await.expect("drive should delete"));
        assert!(repository
            .list()
            .await
            .expect("drives should load")
            .is_empty());
        assert!(!repository
            .delete("c")
            .await
            .expect("missing delete should succeed"));
        repository.close().await;
    }

    std::fs::remove_file(path).expect("test database should be removable");
}
