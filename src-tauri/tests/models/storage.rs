use nexfile_desktop_app_lib::DriveInfo;

#[test]
fn serializes_the_frontend_drive_contract() {
    let drive = DriveInfo {
        drive_id: "c".to_owned(),
        drive_name: "Test SSD".to_owned(),
        partition_name: "System (C:)".to_owned(),
        file_system: "NTFS".to_owned(),
        total_capacity: 1_000,
        system_used: 400,
    };

    assert_eq!(
        serde_json::to_value(drive).expect("drive should serialize"),
        serde_json::json!({
            "driveId": "c",
            "driveName": "Test SSD",
            "partitionName": "System (C:)",
            "fileSystem": "NTFS",
            "totalCapacity": 1_000,
            "systemUsed": 400,
        })
    );
}
