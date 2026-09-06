use crate::models::file_model::FileTypeCount;
use crate::models::storage_model::DriveMetadata;
use crate::types::file_type::FileType;

use crate::services::storage_service::{
    calculate_managed_statistics, read_metadata, write_metadata,
};

#[test]
fn managed_usage_excludes_generated_sidecars_but_counts_imported_json() {
    let directory =
        std::env::temp_dir().join(format!("nexfile-managed-usage-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).expect("test directory should be created");
    std::fs::write(directory.join("image.avif"), [1_u8, 2, 3, 4]).expect("image should be written");
    std::fs::write(directory.join("image.avif.json"), [0_u8; 20])
        .expect("sidecar should be written");
    std::fs::write(directory.join("notes.json"), [5_u8, 6, 7])
        .expect("imported JSON should be written");
    std::fs::write(directory.join("document.txt"), [8_u8, 9]).expect("document should be written");
    std::fs::write(directory.join("document.txt.json"), [10_u8])
        .expect("document sidecar should be written");
    std::fs::write(directory.join(".pending.importing"), [0_u8; 30])
        .expect("temporary file should be written");

    let (files, bytes, counts) =
        calculate_managed_statistics(&directory).expect("usage should be calculated");
    assert_eq!((files, bytes), (3, 9));
    assert_eq!(counts.iter().map(|entry| entry.count).sum::<i64>(), files);

    std::fs::remove_dir_all(&directory).expect("test directory should be removed");
}

#[test]
fn writes_only_total_file_count_in_drive_metadata() {
    let directory =
        std::env::temp_dir().join(format!("nexfile-metadata-counts-{}", uuid::Uuid::new_v4()));
    let path = directory.join("drive_metadata.json");
    let mut metadata = DriveMetadata {
        file_type_counts: crate::FileType::ALL
            .into_iter()
            .map(|file_type| crate::FileTypeCount {
                file_type,
                count: 0,
            })
            .collect(),
        drive_id: "drive-1".to_owned(),
        drive_name: "Test".to_owned(),
        partition_name: "Test".to_owned(),
        app_limit_bytes: None,
        file_count: 3,
        app_used_bytes: 30,
        priority: 1,
        is_mounted: true,
        created_at_ms: 1,
        updated_at_ms: 2,
    };
    let counts = vec![
        FileTypeCount {
            file_type: FileType::Image,
            count: 1,
        },
        FileTypeCount {
            file_type: FileType::Video,
            count: 1,
        },
        FileTypeCount {
            file_type: FileType::Audio,
            count: 0,
        },
        FileTypeCount {
            file_type: FileType::Document,
            count: 1,
        },
        FileTypeCount {
            file_type: FileType::Archive,
            count: 0,
        },
        FileTypeCount {
            file_type: FileType::Other,
            count: 0,
        },
    ];

    metadata.file_type_counts = counts.clone();
    write_metadata(&path, &metadata).expect("drive metadata should be written");

    let read = read_metadata(&path).unwrap();
    assert_eq!(read.file_count, metadata.file_count);
    assert!(read.file_type_counts.iter().all(|entry| entry.count == 0));
    let value = serde_json::from_slice::<serde_json::Value>(
        &std::fs::read(&path).expect("drive metadata should be readable"),
    )
    .expect("drive metadata should be valid JSON");
    assert!(value.get("fileTypeCounts").is_none());
    assert_eq!(value["fileCount"], 3);

    std::fs::remove_dir_all(directory).expect("test directory should be removed");
}
