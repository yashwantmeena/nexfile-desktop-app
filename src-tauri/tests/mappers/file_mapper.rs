use std::path::Path;

use crate::mappers::file_mapper::{file_type_from_path, parse_counts};
use crate::models::file_model::FileTypeCount;
use crate::types::file_type::FileType;
use serde_json::json;

#[test]
fn classifies_common_file_extensions_case_insensitively() {
    for (path, expected) in [
        ("photo.JPG", FileType::Image),
        ("movie.mp4", FileType::Video),
        ("song.flac", FileType::Audio),
        ("report.pdf", FileType::Document),
        ("backup.7z", FileType::Archive),
        ("program.exe", FileType::Other),
        ("README", FileType::Other),
    ] {
        assert_eq!(file_type_from_path(Path::new(path)), expected, "{path}");
    }
}

#[test]
fn writes_counts_as_named_array_entries() {
    let counts = vec![
        FileTypeCount {
            file_type: FileType::Image,
            count: 3,
        },
        FileTypeCount {
            file_type: FileType::Video,
            count: 0,
        },
        FileTypeCount {
            file_type: FileType::Audio,
            count: 0,
        },
        FileTypeCount {
            file_type: FileType::Document,
            count: 0,
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
    let value = serde_json::to_value(&counts).unwrap();
    assert!(value.is_array());
    assert_eq!(value[0], json!({"fileType": "image", "count": 3}));
    assert_eq!(value.as_array().unwrap().len(), 6);
    assert_eq!(parse_counts(value).unwrap(), counts);
}

#[test]
fn accepts_entries_in_any_order_and_defaults_absent_types_to_zero() {
    let counts = parse_counts(json!([
        {"fileType": "video", "count": 2},
        {"fileType": "image", "count": 3}
    ]))
    .unwrap();
    assert_eq!(
        counts
            .iter()
            .find(|entry| entry.file_type == FileType::Video)
            .unwrap()
            .count,
        2
    );
    assert_eq!(
        counts
            .iter()
            .find(|entry| entry.file_type == FileType::Image)
            .unwrap()
            .count,
        3
    );
    assert_eq!(
        counts
            .iter()
            .find(|entry| entry.file_type == FileType::Document)
            .unwrap()
            .count,
        0
    );
}

#[test]
fn rejects_invalid_counts_instead_of_hiding_inconsistencies() {
    for value in [
        json!([{ "fileType": "image", "count": -1 }]),
        json!([{ "fileType": "image", "count": 1.5 }]),
        json!([{ "fileType": "image" }]),
        json!([{ "fileType": "unrecognized", "count": 2 }]),
        json!([{ "fileType": "image", "count": 1 }, { "fileType": "image", "count": 2 }]),
        json!({"imageCount": 3}),
        json!(null),
    ] {
        assert!(parse_counts(value.clone()).is_err(), "{value}");
    }
}
