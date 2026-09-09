use std::time::{SystemTime, UNIX_EPOCH};

use nexfile_desktop_app_lib::{
    DriveMetadata, SqliteBackgroundProcessingRepository, SqliteDatabase, SqliteStorageRepository,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::Row;

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

#[tokio::test]
async fn creates_background_processing_and_drives_tables_in_one_database() {
    let root = test_root();
    let database_path = root.join("nexfile.sqlite3");
    std::fs::create_dir_all(&root).expect("test directory should be created");

    {
        let database = SqliteDatabase::open(&database_path)
            .await
            .expect("database should open");
        let storage = SqliteStorageRepository::new(database.clone());
        let _background = SqliteBackgroundProcessingRepository::new(database);

        storage
            .insert(&DriveMetadata {
                
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
            .await
            .expect("drive should save");
        storage.close().await;
    }

    let pool = SqlitePoolOptions::new()
        .connect_with(SqliteConnectOptions::new().filename(&database_path))
        .await
        .expect("verification database should open");
    let mut table_names = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name != '_sqlx_migrations'",
    )
    .fetch_all(&pool)
    .await
    .expect("table names should load");
    table_names.sort();
    assert_eq!(table_names, vec!["background_processes", "collections", "drives"]);

    let applied_migrations =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = 1")
            .fetch_one(&pool)
            .await
            .expect("migration history should load");
    assert_eq!(applied_migrations, 3);

    let columns = sqlx::query("PRAGMA table_info(background_processes)")
        .fetch_all(&pool)
        .await
        .expect("column names should load")
        .into_iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<Vec<_>>();
    assert_eq!(
        columns,
        vec![
            "process_id",
            "process_type",
            "status",
            "priority",
            "total_items",
            "processed_items",
            "failed_items",
            "remark",
            "collections",
            "created_at_ms",
            "updated_at_ms",
            "started_at_ms",
            "finished_at_ms",
        ]
    );

    let (created_at_ms, updated_at_ms) = sqlx::query_as::<_, (i64, i64)>(
        "INSERT INTO background_processes (
            process_id,
            process_type,
            status
        ) VALUES (?1, ?2, ?3)
        RETURNING created_at_ms, updated_at_ms",
    )
    .bind("process-1")
    .bind("file_indexing")
    .bind("custom_status")
    .fetch_one(&pool)
    .await
    .expect("custom process status should save");
    assert!(created_at_ms > 0);
    assert_eq!(updated_at_ms, created_at_ms);

    pool.close().await;

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[tokio::test]
async fn merges_active_work_by_stage_and_import_collections() {
    let root = test_root();
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let database = SqliteDatabase::open(root.join("nexfile.sqlite3"))
        .await
        .expect("database should open");
    let repository = SqliteBackgroundProcessingRepository::new(database.clone());

    let first = repository
        .acquire_import("import_file", 40, &["Delhi".to_owned()])
        .await
        .expect("first import should create a process");
    let merged = repository
        .acquire_import("import_folder", 60, &["Delhi".to_owned()])
        .await
        .expect("matching collections should reuse the process");
    assert_eq!(merged.process_id, first.process_id);
    assert_eq!(merged.total_items, 100);

    let separate = repository
        .acquire_import("import_file", 10, &["Kedarnath".to_owned()])
        .await
        .expect("different collections should create a process");
    assert_ne!(separate.process_id, first.process_id);

    let ai = repository
        .acquire_stage("image_processing", 40)
        .await
        .expect("AI work should create a process");
    let merged_ai = repository
        .acquire_stage("image_processing", 60)
        .await
        .expect("AI work should reuse its active process");
    assert_eq!(merged_ai.process_id, ai.process_id);
    assert_eq!(merged_ai.total_items, 100);

    repository
        .mark_running(&first.process_id)
        .await
        .expect("process should start");
    repository
        .finish_item(&first.process_id, None)
        .await
        .expect("one item should finish");
    let active = repository.list_active().await.expect("activity should load");
    let updated = active
        .iter()
        .find(|process| process.process_id == first.process_id)
        .expect("partially completed import should remain active");
    assert_eq!(updated.status, nexfile_desktop_app_lib::BackgroundProcessStatus::Running);
    assert_eq!(updated.processed_items, 1);
    assert_eq!(updated.total_items, 100);

    repository.close().await;
    drop(database);
    std::fs::remove_dir_all(root).expect("test directory should be removable");
}


