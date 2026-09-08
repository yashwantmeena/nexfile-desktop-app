
use crate::models::storage_model::*;
use crate::services::file_service::*;
use crate::types::file_type::FileType;
use crate::utils::constants::DRIVE_METADATA_FILE;
use std::path::Path;

fn snapshot() -> DriveMetadata {
    metadata()
}

fn metadata() -> DriveMetadata {
    serde_json::from_value(serde_json::json!({
        "driveId": "drive-1", "driveName": "Test drive", "partitionName": "Test",
        "appLimitBytes": null, "fileCount": 3, "appUsedBytes": 10,
        "createdAtMs": 1, "updatedAtMs": 1
    }))
    .unwrap()
}

#[test]
fn accepts_matching_counts() {
    assert_eq!(validate_drive_counts(&snapshot(), &metadata()), None);
}

#[test]
fn rejects_total_count_mismatch() {
    let mut saved = snapshot();
    let mut data = metadata();
    saved.file_count = 4;
    data.file_count = 3;
    assert!(validate_drive_counts(&saved, &data).is_some());
}

#[test]
fn unavailable_drive_hides_counts_instead_of_claiming_tampering() {
    let result = verify_file_counts(vec![snapshot()], vec![], Path::new("unused"));
    assert!(result.counts.is_none());
    assert!(result.total_count.is_none());
    assert!(result.issues[0].message.contains("unavailable"));
}

#[test]
fn no_saved_drives_returns_verified_zero() {
    let result = verify_file_counts(vec![], vec![], Path::new("unused"));
    assert!(result.counts.is_none());
    assert_eq!(result.total_count, Some(0));
    assert!(result.issues.is_empty());
}

#[test]
fn fetches_all_types_by_filesystem_modified_time_with_pagination() {
    use std::time::{Duration, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!("nexfile-fetch-{}", uuid::Uuid::new_v4()));
    let directory = root.join("nexfile/files");
    std::fs::create_dir_all(&directory).unwrap();
    let saved = metadata();
    std::fs::write(
        root.join("nexfile").join(DRIVE_METADATA_FILE),
        serde_json::to_vec(&saved).unwrap(),
    )
    .unwrap();
    for (name, seconds) in [
        ("old.jpg", 100),
        ("new.pdf", 300),
        ("middle.json", 200),
        ("tie.txt", 200),
    ] {
        let file = std::fs::File::create(directory.join(name)).unwrap();
        file.set_modified(UNIX_EPOCH + Duration::from_secs(seconds))
            .unwrap();
    }
    // A newer AI timestamp must not affect ordering, and sidecars are not files in the result.
    std::fs::write(
        directory.join("old.jpg.json"),
        br#"{"updatedAtMs":9999999999,"classification":{"primary":[],"secondary":[],"tertiary":[]},"searchKeywords":[" landscape ","mountain","Landscape","","snow covered","snow_covered","snow-covered","Some","SOME","some snow","Different","DIFFERENT","different snow","item","Item","ITEM","group","Group","GROUP","two","Three","four","4"]}"#,
    )
    .unwrap();
    std::fs::write(directory.join(".pending.importing"), b"pending").unwrap();
    std::fs::write(
        directory.join("new.pdf.json"),
        br#"{"version":1,"originalName":"Quarterly report.pdf"}"#,
    )
    .unwrap();
    std::fs::create_dir(directory.join("folder")).unwrap();
    let drive = DriveInfo {
        device_id: "test".into(),
        drive_name: "Test drive".into(),
        partition_name: "Test".into(),
        file_system: "test".into(),
        total_bytes: 100,
        system_used_bytes: 10,
        is_system: true,
        mount_point: root.clone(),
    };
    let first = fetch_files(vec![saved.clone()], vec![drive.clone()], &root, None, 0, 2);
    assert!(first.issues.is_empty());
    assert_eq!(first.total_count, 4);
    assert_eq!(first.next_offset, Some(2));
    assert_eq!(
        first
            .files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        ["Quarterly report.pdf", "middle.json"]
    );
    assert_eq!(first.files[0].modified_at_ms, Some(300_000));
    let second = fetch_files(vec![saved.clone()], vec![drive.clone()], &root, None, 2, 2);
    assert_eq!(
        second
            .files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        ["tie.txt", "old.jpg"]
    );
    assert_eq!(second.next_offset, None);
    assert_eq!(second.files[1].file_type, FileType::Image);
    // Match the entire index candidate set before sorting/paginating the filesystem records.
    // Files with matching sidecar text but no index entry must not leak into search.
    let matches = std::collections::HashSet::from([
        (saved.drive_id.clone(), "old".to_owned()),
        (saved.drive_id.clone(), "tie".to_owned()),
        (saved.drive_id.clone(), "deleted".to_owned()),
        ("other-drive".to_owned(), "new".to_owned()),
    ]);
    let matched = fetch_matching_files(vec![saved.clone()], vec![drive.clone()], &root, None, 0, 1, Some(&matches));
    assert_eq!(matched.total_count, 2);
    assert_eq!(matched.next_offset, Some(1));
    assert_eq!(matched.files[0].name, "tie.txt");
    let matched = fetch_matching_files(vec![saved.clone()], vec![drive.clone()], &root, None, 1, 1, Some(&matches));
    assert_eq!(matched.files[0].name, "old.jpg");
    assert_eq!(matched.next_offset, None);
    let matched = fetch_matching_files(vec![saved.clone()], vec![drive.clone()], &root, Some(FileType::Image), 0, 60, Some(&matches));
    assert_eq!(matched.total_count, 1);
    let empty = std::collections::HashSet::new();
    let matched = fetch_matching_files(vec![saved.clone()], vec![drive.clone()], &root, None, 0, 60, Some(&empty));
    assert!(matched.files.is_empty());
    assert_eq!(matched.total_count, 0);
    assert_eq!(matched.next_offset, None);
    let images = fetch_files(
        vec![saved.clone()],
        vec![drive.clone()],
        &root,
        Some(FileType::Image),
        0,
        60,
    );
    assert_eq!(images.total_count, 1);
    assert_eq!(images.files[0].name, "old.jpg");
    assert_eq!(images.files[0].tags, ["landscape", "mountain", "snow", "covered"]);
    for (primary, secondary, expected) in [
        ("visual", Some("nature"), vec!["nature"]),
        (" Visual ", Some("animals"), vec!["animals"]),
        ("visual", None, vec![]),
        ("document", None, vec!["document"]),
    ] {
        let secondary = secondary.into_iter().map(|label| serde_json::json!({
            "label": label, "parentLabel": "visual", "score": 0.9
        })).collect::<Vec<_>>();
        std::fs::write(
            directory.join("old.jpg.json"),
            serde_json::to_vec(&serde_json::json!({
                "classification": {
                    "primary": [{"label": primary, "parentLabel": null, "score": 0.95}],
                    "secondary": secondary,
                    "tertiary": []
                }
            })).unwrap(),
        ).unwrap();
        let page = fetch_files(
            vec![saved.clone()], vec![drive.clone()], &root,
            Some(FileType::Image), 0, 60,
        );
        assert_eq!(page.files[0].categories, expected, "primary: {primary}");
    }
    let beyond = fetch_files(
        vec![saved.clone()],
        vec![drive],
        &root,
        None,
        usize::MAX,
        60,
    );
    assert!(beyond.files.is_empty());
    assert_eq!(beyond.next_offset, None);
    let offline = fetch_files(vec![saved], vec![], &root, None, 0, 60);
    assert!(offline.files.is_empty());
    assert_eq!(offline.issues.len(), 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn reads_drive_metadata_without_modifying_it() {
    let root = std::env::temp_dir().join(format!("nexfile-file-counts-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("nexfile")).unwrap();
    let path = root.join("nexfile").join(DRIVE_METADATA_FILE);
    let data = metadata();
    let mut value = serde_json::to_value(&data).unwrap();
    let bytes = serde_json::to_vec(&value).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let drive = DriveInfo {
        device_id: "test".into(),
        drive_name: "Test drive".into(),
        partition_name: "Test".into(),
        file_system: "test".into(),
        total_bytes: 100,
        system_used_bytes: 10,
        is_system: true,
        mount_point: root.clone(),
    };
    let result = verify_file_counts(vec![snapshot()], vec![drive.clone()], &root);
    assert_eq!(result.total_count, Some(3));
    assert!(result.counts.is_none());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    value["fileCount"] = serde_json::json!(4);
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(verify_file_counts(vec![snapshot()], vec![drive], &root).total_count.is_none());
    std::fs::remove_dir_all(root).unwrap();
}



