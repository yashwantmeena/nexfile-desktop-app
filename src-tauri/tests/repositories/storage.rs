use std::sync::atomic::{AtomicU64, Ordering};

use nexfile_desktop_app_lib::{DriveMetadata, RedbStorageRepository};

static NEXT_TEST_DATABASE: AtomicU64 = AtomicU64::new(0);

fn test_database_path() -> std::path::PathBuf {
    let unique = NEXT_TEST_DATABASE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "nexfile-drives-{}-{unique}.redb",
        std::process::id()
    ))
}

#[test]
fn saves_and_lists_drives() {
    let path = test_database_path();
    let drive = DriveMetadata {
        drive_id: "c".to_owned(),
        drive_name: "Test SSD".to_owned(),
        partition_name: "System (C:)".to_owned(),
        app_limit_bytes: Some(100),
        file_count: 25,
        app_used_bytes: 40,
        priority: 1,
        is_mounted: true,
    };

    {
        let repository = RedbStorageRepository::open(&path).expect("repository should open");
        repository.save(&drive).expect("drive should save");
        assert_eq!(repository.list().expect("drives should load"), vec![drive]);
    }

    std::fs::remove_file(path).expect("test database should be removable");
}
