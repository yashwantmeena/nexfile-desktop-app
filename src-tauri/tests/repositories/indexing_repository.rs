use std::path::PathBuf;

use tantivy::schema::FieldType;

use super::TantivyIndexingRepository;

fn temporary_directory(test_name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("nexfile-{test_name}-{}", uuid::Uuid::new_v4()))
}

#[test]
fn suggests_unique_live_prefix_tags_with_limits_and_refresh() {
    use nexfile_desktop_app_lib::IndexDocument;
    let root = temporary_directory("tag-suggestions");
    let repository = TantivyIndexingRepository::open(&root).unwrap();
    assert!(repository.suggest_tags("be").unwrap().is_empty());
    let mut document = IndexDocument {
        name: String::new(),
        file_id: "one".into(),
        drive_id: "drive".into(),
        created_at_ms: 0,
        updated_at_ms: 0,
        media_type: None,
        size_bytes: 0,
        latitude: None,
        longitude: None,
        object_labels: vec!["beach".into(), "beach ball".into()],
        search_keywords: vec!["beach".into(), "bear".into(), "éclair".into()],
        secondary_labels: vec!["beaver".into()],
        categories: vec!["beauty".into()],
    };
    repository.upsert(document.clone()).unwrap();
    let one = std::collections::HashSet::from([("drive".to_owned(), "one".to_owned())]);
    assert_eq!(repository.search_files(" BE ", "tags", &[]).unwrap(), one);
    assert_eq!(
        repository.search_files("beach b", "tags", &[]).unwrap(),
        one
    );
    assert_eq!(
        repository
            .search_files("", "tags", &["beach".into(), "beauty".into()])
            .unwrap(),
        one
    );
    assert!(repository
        .search_files("be", "tags", &["missing".into()])
        .unwrap()
        .is_empty());
    assert!(repository
        .search_files("be.*", "tags", &[])
        .unwrap()
        .is_empty());
    // Existing tag documents must never be implicitly reindexed as filenames.
    assert!(repository
        .search_files("beach", "name", &[])
        .unwrap()
        .is_empty());
    repository
        .index_filename("drive", "one", "Beach (2026).JPG")
        .unwrap();
    assert_eq!(
        repository.search_files("(2026).jpg", "name", &[]).unwrap(),
        one
    );
    assert_eq!(
        repository
            .search_files("BEACH", "name", &["beauty".into()])
            .unwrap(),
        one
    );
    assert!(repository
        .search_files(".*", "name", &[])
        .unwrap()
        .is_empty());
    assert!(repository
        .search_files("beach", "name", &["missing".into()])
        .unwrap()
        .is_empty());
    assert!(repository.search_files("beach", "invalid", &[]).is_err());
    assert!(repository
        .search_files(&"a".repeat(257), "tags", &[])
        .is_err());
    repository
        .index_filename("drive", "one", "Renamed.jpg")
        .unwrap();
    assert!(repository
        .search_files("beach", "name", &[])
        .unwrap()
        .is_empty());
    assert_eq!(
        repository.search_files("renamed", "name", &[]).unwrap(),
        one
    );
    assert_eq!(
        repository.suggest_tags(" BE ").unwrap(),
        vec!["beach", "beach ball", "bear", "beauty", "beaver"]
    );
    assert_eq!(
        repository.suggest_tags("beach   b").unwrap(),
        vec!["beach ball"]
    );
    assert_eq!(repository.suggest_tags("ÉC").unwrap(), vec!["éclair"]);
    for prefix in ["", "b", "é", "zzz", "be.*"] {
        assert!(repository.suggest_tags(prefix).unwrap().is_empty());
    }
    assert!(repository
        .suggest_tags(&"b".repeat(257))
        .unwrap()
        .is_empty());
    document.object_labels.clear();
    document.secondary_labels.clear();
    document.categories.clear();
    document.search_keywords = (0..600).map(|i| format!("berry{i:03}")).collect();
    repository.upsert(document).unwrap();
    assert!(repository
        .search_files("beach", "tags", &[])
        .unwrap()
        .is_empty());
    let suggestions = repository.suggest_tags("be").unwrap();
    assert_eq!(suggestions.len(), 8);
    assert!(suggestions.iter().all(|value| value.starts_with("berry")));
    assert!(suggestions.windows(2).all(|pair| pair[0] < pair[1]));
    drop(repository);
    let reopened = TantivyIndexingRepository::open(&root).unwrap();
    assert_eq!(reopened.search_files("renamed", "name", &[]).unwrap(), one);
    assert_eq!(reopened.suggest_tags("be").unwrap(), suggestions);
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn opens_index_with_expected_schema() {
    let root = temporary_directory("tantivy-schema");
    let repository =
        TantivyIndexingRepository::open(&root).expect("indexing repository should open");
    let schema = repository.index().schema();
    assert!(!root.join("search-filenames-v1").exists());

    for field_name in [
        "file_id",
        "name",
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
fn keeps_legacy_v3_schema_and_documents_without_reindexing() {
    use tantivy::{doc, Index, TantivyDocument};
    let template_root = temporary_directory("schema-template");
    let template = TantivyIndexingRepository::open(&template_root).unwrap();
    let mut schema_json = serde_json::to_value(template.index().schema()).unwrap();
    schema_json
        .as_array_mut()
        .unwrap()
        .retain(|field| field["name"] != "name");
    let schema: tantivy::schema::Schema = serde_json::from_value(schema_json).unwrap();
    drop(template);
    std::fs::remove_dir_all(template_root).unwrap();

    let root = temporary_directory("legacy-v3");
    let path = root.join("search-index-v3");
    std::fs::create_dir_all(&path).unwrap();
    let index = Index::create_in_dir(&path, schema.clone()).unwrap();
    let mut writer = index
        .writer_with_num_threads::<TantivyDocument>(1, 15_000_000)
        .unwrap();
    writer
        .add_document(doc!(schema.get_field("file_id").unwrap() => "old",
        schema.get_field("drive_id").unwrap() => "drive",
        schema.get_field("search_keywords").unwrap() => "beach"))
        .unwrap();
    writer.commit().unwrap();
    drop(writer);
    drop(index);

    let repository = TantivyIndexingRepository::open(&root).unwrap();
    assert_eq!(repository.index().schema(), schema);
    assert_eq!(
        repository.search_files("bea", "tags", &[]).unwrap().len(),
        1
    );
    assert!(repository
        .search_files("photo", "name", &[])
        .unwrap_err()
        .to_string()
        .contains("name field"));
    repository
        .index_filename("drive", "old", "photo.jpg")
        .unwrap();
    assert_eq!(repository.index().schema(), schema);
    assert_eq!(
        repository.search_files("bea", "tags", &[]).unwrap().len(),
        1
    );
    assert!(!root.join("search-filenames-v1").exists());
    drop(repository);
    std::fs::remove_dir_all(root).unwrap();
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
