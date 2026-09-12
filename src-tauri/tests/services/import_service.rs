
use std::time::{SystemTime, UNIX_EPOCH};

use nexfile_desktop_app_lib::{
    BackgroundProcessStatus, FileProcessingJob, ImageProcessingJob, ImportFileJob, ImportService,
    SqliteBackgroundProcessingRepository, SqliteDatabase, SqliteStorageRepository, StorageService,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

fn test_root() -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "nexfile-import-service-{}-{unique}",
        std::process::id()
    ))
}

#[tokio::test]
async fn persists_each_file_job_across_database_reopen() {
    let root = test_root();
    let database_path = root.join("nexfile.sqlite3");
    let first_file_path = root.join("photo.jpg");
    let second_file_path = root.join("notes.txt");
    std::fs::create_dir_all(&root).expect("test directory should be created");
    std::fs::write(&first_file_path, b"test image").expect("test file should be created");
    std::fs::write(&second_file_path, b"test notes").expect("test file should be created");

    let database = SqliteDatabase::open(&database_path)
        .await
        .expect("database should open");
    let repository = SqliteBackgroundProcessingRepository::new(database.clone());
    let service = ImportService::new(
        repository,
        SqliteStorageRepository::new(database.clone()),
        &database,
        root.clone(),
    )
    .await
    .expect("import queue should be initialized");

    let process = service
        .import_files([&first_file_path, &second_file_path])
        .await
        .expect("file imports should be queued");

    assert_eq!(process.process_type, "import_file");
    assert_eq!(process.status, BackgroundProcessStatus::Queued);
    assert_eq!(process.total_items, 2);
    assert_eq!(process.processed_items, 0);
    assert_eq!(process.failed_items, 0);
    assert!(process.created_at_ms > 0);
    assert_eq!(process.updated_at_ms, process.created_at_ms);

    service.close().await;

    let reopened_database = SqliteDatabase::open(&database_path)
        .await
        .expect("database should reopen");
    let reopened_repository = SqliteBackgroundProcessingRepository::new(reopened_database.clone());
    let reopened_service = ImportService::new(
        reopened_repository,
        SqliteStorageRepository::new(reopened_database.clone()),
        &reopened_database,
        root.clone(),
    )
    .await
    .expect("persistent import queue should reopen");
    let verification_pool = SqlitePoolOptions::new()
        .connect_with(SqliteConnectOptions::new().filename(&database_path))
        .await
        .expect("verification database should open");
    let saved = sqlx::query_as::<_, (String, String, String, i64)>(
        "SELECT process_id, process_type, status, total_items
         FROM background_processes
         WHERE process_id = ?1",
    )
    .bind(&process.process_id)
    .fetch_one(&verification_pool)
    .await
    .expect("background process should be saved");
    assert_eq!(saved.0, process.process_id);
    assert_eq!(saved.1, "import_file");
    assert_eq!(saved.2, "queued");
    assert_eq!(saved.3, 2);

    let queued_jobs = sqlx::query_as::<_, (Vec<u8>,)>(
        "SELECT job
         FROM Jobs
         WHERE job_type = ?1 AND status = 'Pending'
         ORDER BY rowid",
    )
    .bind("file_processing")
    .fetch_all(&verification_pool)
    .await
    .expect("queued import jobs should survive reopening");
    let queued_jobs = queued_jobs
        .into_iter()
        .map(|(job,)| match serde_json::from_slice::<FileProcessingJob>(&job).expect("job should decode") {
            FileProcessingJob::Import(job) => job,
            other => panic!("unexpected file-processing operation: {}", other.operation()),
        })
        .collect::<Vec<_>>();
    assert_eq!(queued_jobs.len(), 2);
    assert_eq!(queued_jobs[0].process_id, process.process_id);
    assert_eq!(queued_jobs[0].file_id.len(), 14);
    assert!(queued_jobs[0]
        .file_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric()));
    assert_eq!(queued_jobs[0].path, first_file_path);
    assert_eq!(queued_jobs[1].process_id, process.process_id);
    assert_eq!(queued_jobs[1].file_id.len(), 14);
    assert!(queued_jobs[1]
        .file_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric()));
    assert_ne!(queued_jobs[0].file_id, queued_jobs[1].file_id);
    assert_eq!(queued_jobs[1].path, second_file_path);

    verification_pool.close().await;
    reopened_service.close().await;
    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[tokio::test]
async fn expands_a_folder_into_individual_file_jobs() {
    let root = test_root();
    let database_path = root.join("nexfile.sqlite3");
    let folder = root.join("selected-folder");
    let nested = folder.join("nested");
    let first_file_path = folder.join("photo.jpg");
    let second_file_path = nested.join("notes.txt");
    std::fs::create_dir_all(&nested).expect("nested directory should be created");
    std::fs::write(&first_file_path, b"test image").expect("test image should be created");
    std::fs::write(&second_file_path, b"test notes").expect("test notes should be created");

    let database = SqliteDatabase::open(&database_path)
        .await
        .expect("database should open");
    let service = ImportService::new(
        SqliteBackgroundProcessingRepository::new(database.clone()),
        SqliteStorageRepository::new(database.clone()),
        &database,
        root.clone(),
    )
    .await
    .expect("import queue should be initialized");

    let process = service
        .import_folder(&folder)
        .await
        .expect("folder files should be queued");
    assert_eq!(process.process_type, "import_folder");
    assert_eq!(process.total_items, 2);

    let verification_pool = SqlitePoolOptions::new()
        .connect_with(SqliteConnectOptions::new().filename(&database_path))
        .await
        .expect("verification database should open");
    let queued_jobs = sqlx::query_as::<_, (Vec<u8>,)>(
        "SELECT job
         FROM Jobs
         WHERE job_type = ?1 AND status = 'Pending'",
    )
    .bind("file_processing")
    .fetch_all(&verification_pool)
    .await
    .expect("folder import jobs should be readable");
    let mut queued_paths = queued_jobs
        .into_iter()
        .map(|(job,)| {
            match serde_json::from_slice::<FileProcessingJob>(&job).expect("job should decode") {
                FileProcessingJob::Import(job) => job.path,
                other => panic!("unexpected file-processing operation: {}", other.operation()),
            }
        })
        .collect::<Vec<_>>();
    queued_paths.sort();
    let mut expected_paths = vec![first_file_path, second_file_path];
    expected_paths.sort();
    assert_eq!(queued_paths, expected_paths);

    verification_pool.close().await;
    service.close().await;
    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[tokio::test]
async fn rejects_a_path_that_is_not_a_file() {
    let root = test_root();
    let database_path = root.join("nexfile.sqlite3");
    std::fs::create_dir_all(&root).expect("test directory should be created");

    let database = SqliteDatabase::open(&database_path)
        .await
        .expect("database should open");
    let repository = SqliteBackgroundProcessingRepository::new(database.clone());
    let service = ImportService::new(
        repository,
        SqliteStorageRepository::new(database.clone()),
        &database,
        root.clone(),
    )
    .await
    .expect("import queue should be initialized");

    let error = service
        .import_files([&root])
        .await
        .expect_err("a directory should not be accepted as a file");
    assert_eq!(error.code(), "VALIDATION_ERROR");
    let empty_error = service
        .import_files(Vec::<std::path::PathBuf>::new())
        .await
        .expect_err("an empty selection should not be accepted");
    assert_eq!(empty_error.code(), "VALIDATION_ERROR");
    let verification_pool = SqlitePoolOptions::new()
        .connect_with(SqliteConnectOptions::new().filename(&database_path))
        .await
        .expect("verification database should open");
    let queued_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM Jobs WHERE job_type = ?1 AND status = 'Pending'",
    )
    .bind("file_processing")
    .fetch_one(&verification_pool)
    .await
    .expect("queue should be readable");
    assert_eq!(queued_count, 0);

    verification_pool.close().await;
    service.close().await;
    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[cfg(target_os = "windows")]
#[tokio::test]
async fn consumes_an_image_into_the_mounted_system_drive_and_queues_processing() {
    let root = test_root();
    let database_path = root.join("nexfile.sqlite3");
    let source = root.join("photo.avip");
    std::fs::create_dir_all(&root).expect("test directory should be created");
    std::fs::write(&source, b"image-content").expect("source should be written");

    let database = SqliteDatabase::open(&database_path)
        .await
        .expect("database should open");
    let storage = StorageService::new(SqliteStorageRepository::new(database.clone()), root.clone());
    let system_partition = storage
        .get_storage_data()
        .await
        .expect("drives should load")
        .drives
        .into_iter()
        .find(|drive| drive.is_system)
        .expect("system drive should exist")
        .partition_name;
    storage
        .mount_drive(None, &system_partition)
        .await
        .expect("system drive should mount");

    let search = nexfile_desktop_app_lib::TantivyIndexingRepository::open(&root).unwrap();
    let imports = ImportService::new(
        SqliteBackgroundProcessingRepository::new(database.clone()),
        SqliteStorageRepository::new(database.clone()),
        &database,
        root.clone(),
    )
    .await
    .expect("import service should initialize")
    .with_search_index(search.clone());
    imports
        .consume(ImportFileJob {
            process_id: "process-1".to_owned(),
            file_id: "abcdefghijklmn".to_owned(),
            path: source,
        })
        .await
        .expect("file should be consumed");

    let destination = root
        .join("nexfile")
        .join("files")
        .join("abcdefghijklmn.avip");
    assert_eq!(
        std::fs::read(&destination).expect("destination should be readable"),
        b"image-content"
    );
    let file_metadata = serde_json::from_slice::<serde_json::Value>(
        &std::fs::read(
            root.join("nexfile")
                .join("files")
                .join("abcdefghijklmn.avip.json"),
        )
        .expect("file metadata should be readable"),
    )
    .expect("file metadata should be valid JSON");
    assert_eq!(file_metadata["version"], 1);
    assert_eq!(file_metadata["name"], "photo.avip");
    assert_eq!(file_metadata["favorite"], false);
    assert!(file_metadata.get("originalName").is_none());
    let matches = search.search_files("PHOTO.AVIP", "name", &[], None).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches.iter().next().unwrap().1, "abcdefghijklmn");
    assert!(search.search_files("photo", "tags", &[], None).unwrap().is_empty());
    let storage_data = storage
        .get_storage_data()
        .await
        .expect("storage metadata should load");
    let mounted = storage_data
        .drives
        .into_iter()
        .find(|drive| drive.is_system && drive.is_mounted)
        .expect("system drive should remain mounted");
    assert_eq!(mounted.file_count, 1);
    assert_eq!(mounted.app_used_bytes, Some(13));
    let drive_metadata = serde_json::from_slice::<serde_json::Value>(
        &std::fs::read(root.join("nexfile").join("drive_metadata.json"))
            .expect("drive metadata should be readable"),
    )
    .expect("drive metadata should be valid JSON");
    assert_eq!(drive_metadata["fileCount"], 1);

    let verification_pool = SqlitePoolOptions::new()
        .connect_with(SqliteConnectOptions::new().filename(&database_path))
        .await
        .expect("verification database should open");
    let (queued_job,) = sqlx::query_as::<_, (Vec<u8>,)>(
        "SELECT job
         FROM Jobs
         WHERE job_type = ?1 AND status = 'Pending'",
    )
    .bind("ai_processing")
    .fetch_one(&verification_pool)
    .await
    .expect("image-processing job should be queued");
    let queued_job = serde_json::from_slice::<ImageProcessingJob>(&queued_job)
        .expect("image-processing job should decode");
    assert!(!queued_job.process_id.is_empty());
    assert_eq!(queued_job.path, destination);

    let document = root.join("Quarterly report.pdf");
    std::fs::write(&document, b"pdf").unwrap();
    imports.consume(ImportFileJob {
        process_id: "process-2".into(), file_id: "nopqrstuvwxyza".into(), path: document,
    }).await.unwrap();
    let matches = search.search_files("report.pdf", "name", &[], None).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches.iter().next().unwrap().1, "nopqrstuvwxyza");

    verification_pool.close().await;
    imports.close().await;
    storage.close().await;
    drop(imports);
    drop(search);
    std::fs::remove_dir_all(root).expect("test directory should be removable");
}



