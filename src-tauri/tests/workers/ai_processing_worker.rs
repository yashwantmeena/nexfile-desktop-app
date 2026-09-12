use crate::models::image_processing_model::ImageProcessingJob;
use crate::repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
use crate::repositories::database_repository::SqliteDatabase;
use crate::repositories::indexing_repository::TantivyIndexingRepository;
use crate::services::image_processing_service::ImageProcessingService;
use crate::services::indexing_service::IndexingService;
use crate::utils::constants::INDEXING_QUEUE;
use crate::utils::constants::{APALIS_MIGRATION_TABLE, IMAGE_PROCESSING_OUTPUT_VERSION};
use crate::workers::ai_processing_worker::*;
use apalis_sqlite::SqliteStorage;

fn test_service(root: &std::path::Path) -> ImageProcessingService {
    ImageProcessingService::new(
        root.join("missing-clip-model"),
        root.join("missing-florence-model"),
        root.join("missing-configs"),
    )
}

fn test_indexing_service(root: &std::path::Path) -> (IndexingService, TantivyIndexingRepository) {
    let repository = TantivyIndexingRepository::open(root.join("app-data"))
        .expect("test search index should open");
    (IndexingService::new(repository.clone()), repository)
}

#[tokio::test]
async fn worker_acknowledges_failed_job_and_processes_next_job() {
    use crate::utils::constants::AI_PROCESSING_QUEUE;
    use apalis::prelude::TaskSink;

    let root = std::env::temp_dir().join(format!("nexfile-worker-retry-{}", uuid::Uuid::new_v4()));
    let database = test_database(&root).await;
    let repository = SqliteBackgroundProcessingRepository::new(database.clone());
    let process = repository
        .acquire_stage("image_processing", 2)
        .await
        .unwrap();
    let invalid = root.join("invalid.avif");
    std::fs::write(&invalid, b"invalid image").unwrap();
    let mut queue = SqliteStorage::<ImageProcessingJob, (), ()>::new_in_queue(
        database.pool(),
        AI_PROCESSING_QUEUE,
    );
    queue
        .push(ImageProcessingJob {
            process_id: process.process_id.clone(),
            path: invalid,
        })
        .await
        .unwrap();
    queue
        .push(ImageProcessingJob {
            process_id: process.process_id.clone(),
            path: root.join("missing.jpg"),
        })
        .await
        .unwrap();
    let worker = AiProcessingWorker::start(
        &database,
        root.join("clip"),
        root.join("florence"),
        root.join("configs"),
        test_indexing_service(&root).0,
    );
    let completed = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            let done: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM Jobs WHERE status = 'Done'")
                .fetch_one(database.pool())
                .await
                .unwrap();
            if done == 2 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    worker.close().await;
    assert!(
        completed.is_ok(),
        "failed job must be acknowledged and the next job completed"
    );
    let saved = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT status, processed_items, failed_items FROM background_processes WHERE process_id = ?1",
    )
    .bind(&process.process_id)
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(saved, ("completed".to_owned(), 2, 1));
    database.close().await;
    std::fs::remove_dir_all(root).unwrap();
}

async fn test_database(root: &std::path::Path) -> SqliteDatabase {
    std::fs::create_dir_all(root).expect("test directory should be created");
    let database = SqliteDatabase::open(root.join("nexfile.sqlite3"))
        .await
        .expect("test database should open");
    let mut migrations = SqliteStorage::migrations();
    migrations.dangerous_set_table_name(APALIS_MIGRATION_TABLE);
    migrations
        .run(database.pool())
        .await
        .expect("queue migrations should run");
    database
}

#[tokio::test]
async fn acknowledges_a_job_when_the_source_file_is_missing() {
    let root = std::env::temp_dir().join(format!(
        "nexfile-missing-image-job-{}",
        uuid::Uuid::new_v4()
    ));
    let database = test_database(&root).await;
    let result = consume_image_processing_job(
        ImageProcessingJob {
            process_id: "process-1".into(),
            path: root.join("missing.avif"),
        },
        test_service(&root),
        test_indexing_service(&root).0,
    )
    .await;

    assert!(result.is_ok());
    database.close().await;
    std::fs::remove_dir_all(root).expect("test directory should be removed");
}

#[tokio::test]
async fn keeps_a_real_processing_failure_retryable() {
    let root = std::env::temp_dir().join(format!(
        "nexfile-invalid-image-job-{}",
        uuid::Uuid::new_v4()
    ));
    let database = test_database(&root).await;
    let path = root.join("invalid.avif");
    std::fs::write(&path, b"not an AVIF image").expect("invalid image should be written");

    let result = consume_image_processing_job(
        ImageProcessingJob {
            process_id: "process-1".into(),
            path,
        },
        test_service(&root),
        test_indexing_service(&root).0,
    )
    .await;

    assert!(result.is_err());
    database.close().await;
    std::fs::remove_dir_all(root).expect("test directory should be removed");
}

#[tokio::test]
async fn indexes_the_processed_json_directly_without_queueing() {
    let root = std::env::temp_dir().join(format!("nexfile-indexing-job-{}", uuid::Uuid::new_v4()));
    let database = test_database(&root).await;
    let storage_root = root.join("storage").join("nexfile");
    let files = storage_root.join("files");
    std::fs::create_dir_all(&files).expect("managed files directory should be created");
    std::fs::write(
        storage_root.join("drive_metadata.json"),
        serde_json::to_vec(&serde_json::json!({
            "driveId": "drive-1",
            "driveName": "Test Drive",
            "partitionName": "Test",
            "appLimitBytes": 1000000,
            "fileCount": 1,
            "appUsedBytes": 12,
            "priority": 1,
            "isMounted": true,
            "createdAtMs": 1,
            "updatedAtMs": 1
        }))
        .expect("drive metadata should serialize"),
    )
    .expect("drive metadata should be written");
    let image_path = files.join("file-1.avif");
    let output_path =
        crate::services::image_processing_service::classification_output_path(&image_path);
    std::fs::write(&image_path, b"cached image").expect("image should be written");
    std::fs::write(
        &output_path,
        serde_json::to_vec(&serde_json::json!({
            "version": IMAGE_PROCESSING_OUTPUT_VERSION,
            "name": "photo.avif",
            "caption": null,
            "ocr": {
                "text": "detected text",
                "rawTextWithRegions": "detected text"
            },
            "objectDetection": null,
            "searchKeywords": [],
            "classification": {
                "primary": [],
                "secondary": [],
                "tertiary": []
            }
        }))
        .expect("cached output should serialize"),
    )
    .expect("cached output should be written");

    let (indexing, indexing_repository) = test_indexing_service(&root);

    consume_image_processing_job(
        ImageProcessingJob {
            process_id: "process-1".into(),
            path: image_path,
        },
        test_service(&root),
        indexing,
    )
    .await
    .expect("processed image should be indexed directly");

    let queued: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM Jobs WHERE job_type = ?1 AND status = 'Pending'")
            .bind(INDEXING_QUEUE)
            .fetch_one(database.pool())
            .await
            .expect("indexing queue should be readable");
    assert_eq!(queued, 0);
    assert_eq!(
        indexing_repository
            .search_files("photo.avif", "name", &[], None)
            .expect("indexed filename should be searchable"),
        std::collections::HashSet::from([("drive-1".to_owned(), "file-1".to_owned())])
    );

    database.close().await;
    std::fs::remove_dir_all(root).expect("test directory should be removed");
}
