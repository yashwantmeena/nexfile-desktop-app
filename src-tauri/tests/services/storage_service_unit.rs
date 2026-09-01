use super::calculate_managed_usage;

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
        .expect("similarly named imported JSON should be written");
    std::fs::write(directory.join(".pending.importing"), [0_u8; 30])
        .expect("temporary file should be written");

    assert_eq!(
        calculate_managed_usage(&directory).expect("usage should be calculated"),
        (4, 10)
    );

    std::fs::remove_dir_all(&directory).expect("test directory should be removed");
}
