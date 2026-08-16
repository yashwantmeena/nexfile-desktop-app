use nexfile_desktop_app_lib::{DriveInfo, DriveMetadata, StorageData, StorageDrive};

#[test]
fn metadata_defaults_to_unmounted() {
    let drive = serde_json::from_value::<DriveMetadata>(serde_json::json!({
        "driveId": "test-drive",
        "deviceId": "legacy-device-id",
        "driveName": "Test SSD",
        "partitionName": "Test (T:)",
        "appLimitBytes": 1_000,
        "fileCount": 25,
        "appUsedBytes": 400,
    }))
    .expect("metadata should deserialize");

    assert_eq!(drive.priority, 0);
    assert!(!drive.is_mounted);
}

#[test]
fn serializes_the_storage_drive_ui_contract() {
    let drive = StorageDrive {
        drive_id: "c".to_owned(),
        device_id: None,
        drive_name: "Test SSD".to_owned(),
        partition_name: "System (C:)".to_owned(),
        file_system: "NTFS".to_owned(),
        total_bytes: 1_000,
        system_used_bytes: Some(400),
        system_used_percent: Some(40),
        app_used_bytes: Some(100),
        app_used_percent: Some(50),
        available_bytes: Some(600),
        app_limit_bytes: Some(200),
        file_count: 25,
        priority: 1,
        is_mounted: true,
        is_connected: true,
        is_system: true,
    };

    assert_eq!(
        serde_json::to_value(drive).expect("storage drive should serialize"),
        serde_json::json!({
            "driveId": "c",
            "deviceId": null,
            "driveName": "Test SSD",
            "partitionName": "System (C:)",
            "fileSystem": "NTFS",
            "totalBytes": 1_000,
            "systemUsedBytes": 400,
            "systemUsedPercent": 40,
            "appUsedBytes": 100,
            "appUsedPercent": 50,
            "availableBytes": 600,
            "appLimitBytes": 200,
            "fileCount": 25,
            "priority": 1,
            "isMounted": true,
            "isConnected": true,
            "isSystem": true,
        })
    );
}

#[test]
fn serializes_the_storage_summary_contract() {
    let connected = StorageDrive {
        drive_id: "c".to_owned(),
        device_id: None,
        drive_name: "Connected".to_owned(),
        partition_name: "System (C:)".to_owned(),
        file_system: "NTFS".to_owned(),
        total_bytes: 1_000,
        system_used_bytes: Some(400),
        system_used_percent: Some(40),
        app_used_bytes: Some(100),
        app_used_percent: Some(50),
        available_bytes: Some(600),
        app_limit_bytes: Some(200),
        file_count: 25,
        priority: 1,
        is_mounted: true,
        is_connected: true,
        is_system: true,
    };
    let unavailable = StorageDrive {
        drive_id: "d".to_owned(),
        device_id: None,
        drive_name: "Unavailable".to_owned(),
        partition_name: "Archive".to_owned(),
        file_system: String::new(),
        total_bytes: 0,
        system_used_bytes: None,
        system_used_percent: None,
        app_used_bytes: Some(50),
        app_used_percent: Some(25),
        available_bytes: None,
        app_limit_bytes: Some(200),
        file_count: 10,
        priority: 2,
        is_mounted: false,
        is_connected: false,
        is_system: false,
    };

    let value = serde_json::to_value(StorageData {
        total_bytes: 1_000,
        available_bytes: 600,
        drives_detected: 1,
        file_indexed: 35,
        app_limit_bytes: 400,
        app_used_bytes: 150,
        drives: vec![connected, unavailable],
    })
    .expect("storage data should serialize");

    assert_eq!(value["totalBytes"], 1_000);
    assert_eq!(value["availableBytes"], 600);
    assert_eq!(value["drivesDetected"], 1);
    assert_eq!(value["fileIndexed"], 35);
    assert_eq!(value["appLimitBytes"], 400);
    assert_eq!(value["appUsedBytes"], 150);
    assert_eq!(value["drives"].as_array().map(Vec::len), Some(2));
}

#[test]
fn serializes_and_deserializes_a_saved_drive() {
    let drive = DriveMetadata {
        drive_id: "c".to_owned(),
        drive_name: "Test SSD".to_owned(),
        partition_name: "System (C:)".to_owned(),
        app_limit_bytes: Some(100),
        file_count: 25,
        app_used_bytes: 40,
        priority: 1,
        is_mounted: true,
    };

    let value = serde_json::to_value(&drive).expect("drive should serialize");
    assert_eq!(value["driveId"], "c");
    assert!(value.get("deviceId").is_none());
    assert_eq!(value["appLimitBytes"], 100);
    assert_eq!(value["fileCount"], 25);
    assert_eq!(value["appUsedBytes"], 40);
    assert_eq!(value["priority"], 1);
    assert_eq!(value["isMounted"], true);

    assert_eq!(
        serde_json::from_value::<DriveMetadata>(value).expect("drive should deserialize"),
        drive
    );
}

#[test]
fn serializes_the_frontend_drive_contract() {
    let drive = DriveInfo {
        device_id: "device-c".to_owned(),
        drive_name: "Test SSD".to_owned(),
        partition_name: "System (C:)".to_owned(),
        file_system: "NTFS".to_owned(),
        total_bytes: 1_000,
        system_used_bytes: 400,
        is_system: true,
        mount_point: "C:\\".into(),
    };

    assert_eq!(
        serde_json::to_value(drive).expect("drive should serialize"),
        serde_json::json!({
            "deviceId": "device-c",
            "driveName": "Test SSD",
            "partitionName": "System (C:)",
            "fileSystem": "NTFS",
            "totalBytes": 1_000,
            "systemUsedBytes": 400,
            "isSystem": true,
        })
    );
}
