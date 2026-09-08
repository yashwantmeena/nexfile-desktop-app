use std::sync::atomic::{AtomicU64, Ordering};

use nexfile_desktop_app_lib::{DriveMetadata, SqliteStorageRepository};

static NEXT_TEST_DATABASE: AtomicU64 = AtomicU64::new(0);

fn test_database_path() -> std::path::PathBuf {
    let unique = NEXT_TEST_DATABASE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "nexfile-drives-{}-{unique}.sqlite3",
        std::process::id()
    ))
}

#[tokio::test]
async fn saves_and_lists_drives() {
    let path = test_database_path();
    let mut drive = DriveMetadata {
        
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
        let inserted = repository.insert(&drive).await.expect("drive should save");
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
        let updated = repository
            .update(&drive, |_| Ok(()))
            .await
            .expect("drive update should save");
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

