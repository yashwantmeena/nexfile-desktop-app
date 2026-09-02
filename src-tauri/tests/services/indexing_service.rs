use nexfile_desktop_app_lib::{
    ClassificationPrediction, DriveMetadata, ImageBoundingBox, ImageClassificationOutput,
    ImageLocation, ImageMetadata, ImageObjectDetection, ImageObjectDetectionOutput,
    ImageProcessingOutput, IndexingJob, IndexingService, TantivyIndexingRepository,
};
use tantivy::collector::{Count, TopDocs};
use tantivy::query::TermQuery;
use tantivy::schema::IndexRecordOption;
use tantivy::{Document, TantivyDocument, Term};

fn prediction(label: &str) -> ClassificationPrediction {
    ClassificationPrediction {
        label: label.to_owned(),
        parent_label: None,
        score: 0.9,
    }
}

#[tokio::test]
async fn stores_searchable_image_fields_in_tantivy() {
    let root =
        std::env::temp_dir().join(format!("nexfile-indexing-service-{}", uuid::Uuid::new_v4()));
    let app_data = root.join("app-data");
    let storage_root = root.join("storage").join("nexfile");
    let files = storage_root.join("files");
    std::fs::create_dir_all(&files).expect("managed files directory should be created");

    let drive = DriveMetadata {
        drive_id: "drive-1".to_owned(),
        drive_name: "Test Drive".to_owned(),
        partition_name: "Test".to_owned(),
        app_limit_bytes: Some(1_000_000),
        file_count: 1,
        app_used_bytes: 5,
        priority: 1,
        is_mounted: true,
        created_at_ms: 1,
        updated_at_ms: 1,
    };
    std::fs::write(
        storage_root.join("drive_metadata.json"),
        serde_json::to_vec(&drive).expect("drive metadata should serialize"),
    )
    .expect("drive metadata should be written");

    let image_path = files.join("AbC123.jpg");
    let sidecar_path = files.join("AbC123.jpg.json");
    std::fs::write(&image_path, b"image").expect("managed image should be written");
    let output = ImageProcessingOutput {
        version: 18,
        created_at_ms: 1_788_331_200_000,
        updated_at_ms: 1_788_331_300_000,
        metadata: ImageMetadata {
            media_type: Some("image/jpeg".to_owned()),
            size_bytes: 245_000,
            width: 100,
            height: 100,
            location: Some(ImageLocation {
                latitude: 12.971_599,
                longitude: 77.594_566,
            }),
            perceptual_hash: String::new(),
        },
        caption: Some("A red fox on a forest trail".to_owned()),
        ocr: None,
        object_detection: Some(ImageObjectDetectionOutput {
            raw_text: String::new(),
            image_width: 100,
            image_height: 100,
            detections: vec![ImageObjectDetection {
                label: "Red Fox".to_owned(),
                bounding_box: ImageBoundingBox {
                    x_min: 0,
                    y_min: 0,
                    x_max: 10,
                    y_max: 10,
                },
            }],
        }),
        search_keywords: vec!["Forest Trail".to_owned(), "red fox".to_owned()],
        classification: ImageClassificationOutput {
            primary: vec![prediction("visual")],
            secondary: vec![prediction("Nature & Outdoors")],
            tertiary: vec![],
        },
    };
    std::fs::write(
        &sidecar_path,
        serde_json::to_vec(&output).expect("processing output should serialize"),
    )
    .expect("processing sidecar should be written");

    let repository =
        TantivyIndexingRepository::open(&app_data).expect("indexing repository should open");
    let indexing = IndexingService::new(repository.clone());
    indexing
        .process(IndexingJob { path: sidecar_path })
        .await
        .expect("sidecar should be indexed");

    let reader = repository
        .index()
        .reader()
        .expect("index reader should open");
    let searcher = reader.searcher();
    let schema = repository.index().schema();
    for (field, value) in [
        ("object_labels", "red fox"),
        ("search_keywords", "forest trail"),
        ("search_keywords", "red fox"),
        ("secondary_labels", "nature & outdoors"),
        ("categories", "visual"),
        ("categories", "nature & outdoors"),
    ] {
        let query = TermQuery::new(
            Term::from_field_text(
                schema.get_field(field).expect("search field should exist"),
                value,
            ),
            IndexRecordOption::Basic,
        );
        assert_eq!(
            searcher
                .search(&query, &Count)
                .expect("tag query should run"),
            1,
            "missing indexed value {value}"
        );
    }

    let file_query = TermQuery::new(
        Term::from_field_text(
            schema
                .get_field("file_id")
                .expect("file ID field should exist"),
            "AbC123",
        ),
        IndexRecordOption::Basic,
    );
    let (_, address) = searcher
        .search(&file_query, &TopDocs::with_limit(1).order_by_score())
        .expect("file query should run")
        .into_iter()
        .next()
        .expect("indexed file should be found");
    let document = searcher
        .doc::<TantivyDocument>(address)
        .expect("indexed document should load");
    let stored =
        serde_json::from_str::<serde_json::Value>(&document.to_json(&repository.index().schema()))
            .expect("stored document should be JSON");
    assert_eq!(stored["drive_id"][0], "drive-1");
    assert_eq!(stored["created_at_ms"][0], 1_788_331_200_000_u64);
    assert_eq!(stored["updated_at_ms"][0], 1_788_331_300_000_u64);
    assert_eq!(stored["media_type"][0], "image/jpeg");
    assert_eq!(stored["size_bytes"][0], 245_000);
    assert_eq!(stored["latitude"][0], 12.971_599);
    assert_eq!(stored["longitude"][0], 77.594_566);

    drop(document);
    drop(searcher);
    drop(reader);
    drop(indexing);
    drop(repository);
    std::fs::remove_dir_all(root).expect("test directory should be removable");
}
