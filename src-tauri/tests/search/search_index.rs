use std::path::PathBuf;

use tantivy::schema::FieldType;

use super::SearchIndex;

fn temporary_directory(test_name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("nexfile-{test_name}-{}", uuid::Uuid::new_v4()))
}

#[test]
fn opens_search_index_with_expected_schema() {
    let root = temporary_directory("tantivy-schema");
    let search = SearchIndex::open(&root).expect("search index should open");
    let schema = search.index().schema();

    for field_name in [
        "file_id",
        "drive_id",
        "name",
        "extension",
        "tags",
        "caption",
        "ocr",
        "modified_at_ms",
        "size_bytes",
    ] {
        assert!(schema.get_field(field_name).is_ok(), "missing {field_name}");
    }
    let FieldType::Str(tag_options) = schema.get_field_entry(search.fields().tags).field_type()
    else {
        panic!("tags should be a text field");
    };
    assert_eq!(
        tag_options
            .get_indexing_options()
            .expect("tags should be indexed")
            .tokenizer(),
        "raw"
    );

    drop(search);
    std::fs::remove_dir_all(root).expect("temporary index should be removable");
}

#[test]
fn reopens_an_existing_search_index() {
    let root = temporary_directory("tantivy-reopen");
    let first = SearchIndex::open(&root).expect("search index should be created");
    drop(first);

    SearchIndex::open(&root).expect("existing search index should reopen");
    std::fs::remove_dir_all(root).expect("temporary index should be removable");
}

#[test]
fn ignores_an_index_from_an_older_schema_generation() {
    let root = temporary_directory("tantivy-schema-upgrade");
    let legacy = root.join("search-index");
    std::fs::create_dir_all(&legacy).expect("legacy index directory should be created");
    std::fs::write(legacy.join("meta.json"), b"incompatible legacy index")
        .expect("legacy index fixture should be written");

    SearchIndex::open(&root).expect("a schema upgrade should create a fresh index");

    assert!(legacy.join("meta.json").is_file());
    assert!(root.join("search-index-v2").join("meta.json").is_file());
    std::fs::remove_dir_all(root).expect("temporary index should be removable");
}
