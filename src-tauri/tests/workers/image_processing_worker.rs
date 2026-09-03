use crate::models::{image_processing_model::ImageProcessingJob, indexing_model::IndexingJob};
use crate::repositories::database_repository::SqliteDatabase;
use crate::services::image_processing_service::ImageProcessingService;
use crate::utils::constants::INDEXING_QUEUE;
use crate::utils::constants::{APALIS_MIGRATION_TABLE, IMAGE_PROCESSING_OUTPUT_VERSION};
use crate::workers::image_processing_worker::*;
use apalis_sqlite::SqliteStorage;

fn test_service(root: &std::path::Path) -> ImageProcessingService {
    ImageProcessingService::new(
        root.join("missing-clip-model"),
        root.join("missing-florence-model"),
        root.join("missing-configs"),
    )
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
            path: root.join("missing.avif"),
        },
        test_service(&root),
        database.pool().clone(),
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
        ImageProcessingJob { path },
        test_service(&root),
        database.pool().clone(),
    )
    .await;

    assert!(result.is_err());
    database.close().await;
    std::fs::remove_dir_all(root).expect("test directory should be removed");
}

#[tokio::test]
async fn publishes_the_processed_json_path_for_indexing() {
    let root = std::env::temp_dir().join(format!("nexfile-indexing-job-{}", uuid::Uuid::new_v4()));
    let database = test_database(&root).await;
    let image_path = root.join("photo.avif");
    let output_path =
        crate::services::image_processing_service::classification_output_path(&image_path);
    std::fs::write(&image_path, b"cached image").expect("image should be written");
    std::fs::write(
        &output_path,
        serde_json::to_vec(&serde_json::json!({
            "version": IMAGE_PROCESSING_OUTPUT_VERSION,
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

    consume_image_processing_job(
        ImageProcessingJob { path: image_path },
        test_service(&root),
        database.pool().clone(),
    )
    .await
    .expect("processed image should publish an indexing job");

    let (job,) = sqlx::query_as::<_, (Vec<u8>,)>(
        "SELECT job FROM Jobs WHERE job_type = ?1 AND status = 'Pending'",
    )
    .bind(INDEXING_QUEUE)
    .fetch_one(database.pool())
    .await
    .expect("indexing job should be queued");
    let job = serde_json::from_slice::<IndexingJob>(&job).expect("indexing job should decode");
    assert_eq!(job.path, output_path);

    database.close().await;
    std::fs::remove_dir_all(root).expect("test directory should be removed");
}
