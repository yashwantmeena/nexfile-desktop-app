use nexfile_desktop_app_lib::{
    ClassificationPrediction, DriveMetadata, ImageBoundingBox, ImageClassificationOutput,
    ImageObjectDetection, ImageObjectDetectionOutput, ImageProcessingOutput, IndexingJob,
    IndexingService, SearchIndex,
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
async fn stores_namespaced_tags_in_tantivy() {
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
        created_at_ms: 0,
        updated_at_ms: 0,
        metadata: Default::default(),
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

    let search = SearchIndex::open(&app_data).expect("search index should open");
    let indexing = IndexingService::new(&search).expect("indexing service should open");
    indexing
        .process(IndexingJob { path: sidecar_path })
        .await
        .expect("sidecar should be indexed");

    let reader = search.index().reader().expect("index reader should open");
    let searcher = reader.searcher();
    let fields = search.fields();
    for tag in [
        "ct::nature_outdoors",
        "ob::red_fox",
        "tg::nature_outdoors",
        "tg::red_fox",
        "tg::forest_trail",
    ] {
        let query = TermQuery::new(
            Term::from_field_text(fields.tags, tag),
            IndexRecordOption::Basic,
        );
        assert_eq!(
            searcher
                .search(&query, &Count)
                .expect("tag query should run"),
            1,
            "missing indexed tag {tag}"
        );
    }

    let file_query = TermQuery::new(
        Term::from_field_text(fields.file_id, "AbC123"),
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
        serde_json::from_str::<serde_json::Value>(&document.to_json(&search.index().schema()))
            .expect("stored document should be JSON");
    assert_eq!(stored["drive_id"][0], "drive-1");
    assert_eq!(stored["name"][0], "AbC123.jpg");
    assert_eq!(stored["extension"][0], "jpg");
    assert_eq!(stored["size_bytes"][0], 5);

    drop(document);
    drop(searcher);
    drop(reader);
    drop(indexing);
    drop(search);
    std::fs::remove_dir_all(root).expect("test directory should be removable");
}
