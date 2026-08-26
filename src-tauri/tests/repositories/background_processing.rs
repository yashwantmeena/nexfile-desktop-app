use std::time::{SystemTime, UNIX_EPOCH};

use nexfile_desktop_app_lib::{
    DriveMetadata, RedbBackgroundProcessingRepository, RedbDatabase, RedbStorageRepository,
};
use redb::{Database, ReadableDatabase, TableHandle};

fn test_root() -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "nexfile-background-processing-repository-{}-{unique}",
        std::process::id()
    ))
}

#[test]
fn creates_background_processing_and_drives_tables_in_one_database() {
    let root = test_root();
    let database_path = root.join("nexfile.redb");
    std::fs::create_dir_all(&root).expect("test directory should be created");

    {
        let database = RedbDatabase::open(&database_path).expect("database should open");
        let storage =
            RedbStorageRepository::new(database.clone()).expect("storage table should open");
        let _background = RedbBackgroundProcessingRepository::new(database)
            .expect("background processing table should open");

        storage
            .save(&DriveMetadata {
                drive_id: "drive-1".to_owned(),
                drive_name: "Test drive".to_owned(),
                partition_name: "Test".to_owned(),
                app_limit_bytes: Some(1_000),
                file_count: 5,
                app_used_bytes: 100,
                priority: 1,
                is_mounted: true,
                created_at_ms: 0,
                updated_at_ms: 0,
            })
            .expect("drive should save");
    }

    let database = Database::open(&database_path).expect("raw database should reopen");
    let read = database.begin_read().expect("read transaction should open");
    let mut table_names = read
        .list_tables()
        .expect("tables should list")
        .map(|table| table.name().to_owned())
        .collect::<Vec<_>>();
    table_names.sort();
    assert_eq!(table_names, vec!["background_processing", "drives"]);
    drop(read);
    drop(database);

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}
