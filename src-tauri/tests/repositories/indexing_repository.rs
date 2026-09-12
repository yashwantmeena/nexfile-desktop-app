use std::path::PathBuf;

use tantivy::schema::FieldType;

use super::TantivyIndexingRepository;

fn temporary_directory(test_name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("nexfile-{test_name}-{}", uuid::Uuid::new_v4()))
}

#[test]
fn counts_indexed_matches_across_types_search_and_tags() {
    use nexfile_desktop_app_lib::{FileType, IndexDocument};
    let root = temporary_directory("filtered-counts");
    let repository = TantivyIndexingRepository::open(&root).unwrap();
    assert_eq!(
        repository
            .indexed_results("", "tags", &[], None)
            .unwrap()
            .1
            .total_count,
        Some(0)
    );
    let mut source = IndexDocument {
        name: "Beach.jpg".into(),
        file_id: "photo".into(),
        drive_id: "drive-a".into(),
        created_at_ms: 100,
        updated_at_ms: 100,
        media_type: Some("image/jpeg".into()),
        size_bytes: 1,
        latitude: None,
        longitude: None,
        object_labels: vec!["beach".into()],
        search_keywords: vec!["beach".into()],
        secondary_labels: vec![],
        categories: vec!["travel".into()],
        collection_ids: vec!["local-delhi-a".into()],
        favorite: true,
    };
    repository.upsert(source.clone()).unwrap();
    source.file_id = "report".into();
    source.drive_id = "drive-b".into();
    source.name = "Beach report.pdf".into();
    source.collection_ids = vec!["local-delhi-b".into()];
    source.favorite = false;
    source.media_type = Some("application/pdf".into());
    source.updated_at_ms = 200;
    repository.upsert(source.clone()).unwrap();
    repository
        .index_filename("drive-a", "song", "Music.mp3", &[], false)
        .unwrap();
    let (files, summary) = repository.indexed_results("", "tags", &[], None).unwrap();
    assert_eq!(files.len(), 3);
    assert_eq!(summary.total_count, Some(3));
    let (favorites, favorite_summary) = repository
        .indexed_results_filtered("", "tags", &[], None, true)
        .unwrap();
    assert_eq!(
        favorites,
        std::collections::HashSet::from([("drive-a".to_owned(), "photo".to_owned())])
    );
    assert_eq!(favorite_summary.total_count, Some(1));
    let delhi_ids = vec![
        ("drive-a".to_owned(), "local-delhi-a".to_owned()),
        ("drive-b".to_owned(), "local-delhi-b".to_owned()),
    ];
    assert_eq!(
        repository
            .indexed_results("", "tags", &[], Some(&delhi_ids))
            .unwrap()
            .1
            .total_count,
        Some(2)
    );
    let wrong_drive = vec![("drive-a".to_owned(), "local-delhi-b".to_owned())];
    assert!(repository
        .search_files("", "tags", &[], Some(&wrong_drive))
        .unwrap()
        .is_empty());
    assert!(repository
        .search_files("", "tags", &[], Some(&[]))
        .unwrap()
        .is_empty());
    let counts = summary.counts.unwrap();
    assert_eq!(counts.len(), 6);
    for kind in [FileType::Image, FileType::Document, FileType::Audio] {
        assert_eq!(
            counts
                .iter()
                .find(|entry| entry.file_type == kind)
                .unwrap()
                .count,
            1
        );
    }
    assert_eq!(
        repository
            .indexed_results("bea", "tags", &[], None)
            .unwrap()
            .1
            .total_count,
        Some(2)
    );
    assert_eq!(
        repository
            .indexed_results("bea", "tags", &["travel".into()], None)
            .unwrap()
            .1
            .total_count,
        Some(2)
    );
    assert_eq!(
        repository
            .indexed_results("REPORT.PDF", "name", &["beach".into()], None)
            .unwrap()
            .1
            .total_count,
        Some(1)
    );
    assert_eq!(
        repository
            .indexed_results("REPORT.PDF", "name", &[], None)
            .unwrap()
            .1
            .total_count,
        Some(1)
    );
    assert_eq!(
        repository
            .indexed_results("bea", "tags", &["missing".into()], None)
            .unwrap()
            .1
            .total_count,
        Some(0)
    );
    repository.upsert(source).unwrap();
    assert_eq!(
        repository
            .indexed_results("", "tags", &[], None)
            .unwrap()
            .1
            .total_count,
        Some(3)
    );
    drop(repository);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn deletes_every_indexed_document_for_a_removed_drive() {
    let root = temporary_directory("delete-drive");
    let repository = TantivyIndexingRepository::open(&root).unwrap();
    repository
        .index_filename("removed-drive", "one", "one.jpg", &[], false)
        .unwrap();
    repository
        .index_filename("removed-drive", "two", "two.jpg", &[], false)
        .unwrap();
    repository
        .index_filename("retained-drive", "three", "three.jpg", &[], false)
        .unwrap();

    repository.delete_drive("removed-drive").unwrap();

    assert_eq!(
        repository.search_files("", "tags", &[], None).unwrap(),
        std::collections::HashSet::from([("retained-drive".to_owned(), "three".to_owned())])
    );
    drop(repository);
    std::fs::remove_dir_all(root).unwrap();
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
        collection_ids: vec![],
        favorite: false,
    };
    repository.upsert(document.clone()).unwrap();
    let one = std::collections::HashSet::from([("drive".to_owned(), "one".to_owned())]);
    assert_eq!(
        repository.search_files(" BE ", "tags", &[], None).unwrap(),
        one
    );
    assert_eq!(
        repository
            .search_files("beach b", "tags", &[], None)
            .unwrap(),
        one
    );
    assert_eq!(
        repository
            .search_files("", "tags", &["beach".into(), "beauty".into()], None)
            .unwrap(),
        one
    );
    assert!(repository
        .search_files("be", "tags", &["missing".into()], None)
        .unwrap()
        .is_empty());
    assert!(repository
        .search_files("be.*", "tags", &[], None)
        .unwrap()
        .is_empty());
    // Existing tag documents must never be implicitly reindexed as filenames.
    assert!(repository
        .search_files("beach", "name", &[], None)
        .unwrap()
        .is_empty());
    repository
        .index_filename("drive", "one", "Beach (2026).JPG", &[], false)
        .unwrap();
    assert_eq!(
        repository
            .search_files("(2026).jpg", "name", &[], None)
            .unwrap(),
        one
    );
    assert_eq!(
        repository
            .search_files("BEACH", "name", &["beauty".into()], None)
            .unwrap(),
        one
    );
    assert!(repository
        .search_files(".*", "name", &[], None)
        .unwrap()
        .is_empty());
    assert!(repository
        .search_files("beach", "name", &["missing".into()], None)
        .unwrap()
        .is_empty());
    assert!(repository
        .search_files("beach", "invalid", &[], None)
        .is_err());
    assert!(repository
        .search_files(&"a".repeat(257), "tags", &[], None)
        .is_err());
    repository
        .index_filename("drive", "one", "Renamed.jpg", &[], false)
        .unwrap();
    assert!(repository
        .search_files("beach", "name", &[], None)
        .unwrap()
        .is_empty());
    assert_eq!(
        repository
            .search_files("renamed", "name", &[], None)
            .unwrap(),
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
    repository
        .index_file_metadata(
            "drive",
            "one",
            "Renamed.jpg",
            &[],
            &["sunset".into()],
            false,
        )
        .unwrap();
    assert_eq!(
        repository
            .search_files("sunset", "tags", &[], None)
            .unwrap(),
        one
    );
    assert!(repository
        .search_files("beach", "tags", &[], None)
        .unwrap()
        .is_empty());
    document.object_labels.clear();
    document.secondary_labels.clear();
    document.categories.clear();
    document.search_keywords = (0..600).map(|i| format!("berry{i:03}")).collect();
    repository.upsert(document).unwrap();
    assert!(repository
        .search_files("beach", "tags", &[], None)
        .unwrap()
        .is_empty());
    let suggestions = repository.suggest_tags("be").unwrap();
    assert_eq!(suggestions.len(), 8);
    assert!(suggestions.iter().all(|value| value.starts_with("berry")));
    assert!(suggestions.windows(2).all(|pair| pair[0] < pair[1]));
    drop(repository);
    let reopened = TantivyIndexingRepository::open(&root).unwrap();
    assert_eq!(
        reopened.search_files("renamed", "name", &[], None).unwrap(),
        one
    );
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
        "collection_ids",
        "favorite",
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
fn migrates_v3_documents_with_favorites_off_by_default() {
    use tantivy::collector::Count;
    use tantivy::query::AllQuery;
    use tantivy::schema::{Schema, STORED, STRING};
    use tantivy::{Index, TantivyDocument};

    let root = temporary_directory("favorite-index-migration");
    let old_path = root.join("search-index-v3");
    std::fs::create_dir_all(&old_path).unwrap();
    let mut schema = Schema::builder();
    let drive_id = schema.add_text_field("drive_id", STRING | STORED);
    let file_id = schema.add_text_field("file_id", STRING | STORED);
    let name = schema.add_text_field("name", STRING | STORED);
    let old = Index::create_in_dir(&old_path, schema.build()).unwrap();
    let mut writer = old.writer::<TantivyDocument>(15_000_000).unwrap();
    let mut document = TantivyDocument::default();
    document.add_text(drive_id, "drive");
    document.add_text(file_id, "photo");
    document.add_text(name, "photo.jpg");
    writer.add_document(document).unwrap();
    writer.commit().unwrap();
    drop(writer);
    drop(old);

    let repository = TantivyIndexingRepository::open(&root).unwrap();
    assert!(root.join("search-index-v4").is_dir());
    assert_eq!(
        repository.search_files("photo", "name", &[], None).unwrap(),
        std::collections::HashSet::from([("drive".to_owned(), "photo".to_owned())])
    );
    assert!(repository
        .indexed_results_filtered("", "tags", &[], None, true)
        .unwrap()
        .0
        .is_empty());
    let reader = repository.index().reader().unwrap();
    assert_eq!(reader.searcher().search(&AllQuery, &Count).unwrap(), 1);

    drop(reader);
    drop(repository);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_an_outdated_schema() {
    use tantivy::Index;
    let template_root = temporary_directory("schema-template");
    let template = TantivyIndexingRepository::open(&template_root).unwrap();
    let mut schema_json = serde_json::to_value(template.index().schema()).unwrap();
    schema_json
        .as_array_mut()
        .unwrap()
        .retain(|field| field["name"] != "collection_ids");
    let schema: tantivy::schema::Schema = serde_json::from_value(schema_json).unwrap();
    drop(template);
    std::fs::remove_dir_all(template_root).unwrap();

    let root = temporary_directory("outdated-schema");
    let path = root.join("search-index-v4");
    std::fs::create_dir_all(&path).unwrap();
    drop(Index::create_in_dir(&path, schema).unwrap());

    let error = TantivyIndexingRepository::open(&root)
        .err()
        .expect("an outdated schema should be rejected");
    assert!(error.to_string().contains("unsupported schema"));
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
