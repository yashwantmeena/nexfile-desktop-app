use super::*;

fn test_service(root: &std::path::Path) -> ImageProcessingService {
    ImageProcessingService::new(
        root.join("missing-clip-model"),
        root.join("missing-florence-model"),
        root.join("missing-configs"),
    )
}

#[tokio::test]
async fn acknowledges_a_job_when_the_source_file_is_missing() {
    let root = std::env::temp_dir().join(format!(
        "nexfile-missing-image-job-{}",
        uuid::Uuid::new_v4()
    ));
    let result = consume_image_processing_job(
        ImageProcessingJob {
            path: root.join("missing.avif"),
        },
        test_service(&root),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn keeps_a_real_processing_failure_retryable() {
    let root = std::env::temp_dir().join(format!(
        "nexfile-invalid-image-job-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let path = root.join("invalid.avif");
    std::fs::write(&path, b"not an AVIF image").expect("invalid image should be written");

    let result =
        consume_image_processing_job(ImageProcessingJob { path }, test_service(&root)).await;

    assert!(result.is_err());
    std::fs::remove_dir_all(root).expect("test directory should be removed");
}
