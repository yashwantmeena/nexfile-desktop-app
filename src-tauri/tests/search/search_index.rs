use std::path::PathBuf;

use nexfile_desktop_app_lib::SearchIndex;

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
