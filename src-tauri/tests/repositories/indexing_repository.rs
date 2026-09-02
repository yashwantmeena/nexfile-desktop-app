use std::path::PathBuf;

use tantivy::schema::FieldType;

use super::TantivyIndexingRepository;

fn temporary_directory(test_name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("nexfile-{test_name}-{}", uuid::Uuid::new_v4()))
}

#[test]
fn opens_index_with_expected_schema() {
    let root = temporary_directory("tantivy-schema");
    let repository =
        TantivyIndexingRepository::open(&root).expect("indexing repository should open");
    let schema = repository.index().schema();

    for field_name in [
        "file_id",
        "drive_id",
        "created_at_ms",
        "updated_at_ms",
        "media_type",
        "size_bytes",
        "latitude",
        "longitude",
        "object_labels",
        "search_keywords",
        "secondary_labels",
        "categories",
    ] {
        assert!(schema.get_field(field_name).is_ok(), "missing {field_name}");
    }
    let FieldType::Str(options) = schema
        .get_field_entry(
            schema
                .get_field("search_keywords")
                .expect("search keywords field should exist"),
        )
        .field_type()
    else {
        panic!("search keywords should be a text field");
    };
    assert_eq!(
        options
            .get_indexing_options()
            .expect("search keywords should be indexed")
            .tokenizer(),
        "raw"
    );

    drop(repository);
    std::fs::remove_dir_all(root).expect("temporary index should be removable");
}

#[test]
fn reopens_an_existing_index() {
    let root = temporary_directory("tantivy-reopen");
    let first =
        TantivyIndexingRepository::open(&root).expect("indexing repository should be created");
    drop(first);

    TantivyIndexingRepository::open(&root).expect("existing indexing repository should reopen");
    std::fs::remove_dir_all(root).expect("temporary index should be removable");
}

#[test]
fn ignores_an_index_from_an_older_schema_generation() {
    let root = temporary_directory("tantivy-schema-upgrade");
    let legacy = root.join("search-index");
    std::fs::create_dir_all(&legacy).expect("legacy index directory should be created");
    std::fs::write(legacy.join("meta.json"), b"incompatible legacy index")
        .expect("legacy index fixture should be written");

    TantivyIndexingRepository::open(&root).expect("a schema upgrade should create a fresh index");

    assert!(legacy.join("meta.json").is_file());
    assert!(root.join("search-index-v3").join("meta.json").is_file());
    std::fs::remove_dir_all(root).expect("temporary index should be removable");
}
