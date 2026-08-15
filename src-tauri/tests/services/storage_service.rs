use nexfile_desktop_app_lib::StorageService;

#[test]
fn returns_connected_drives_with_the_required_information() {
    let drives = StorageService.get_available_drives();
    assert!(!drives.is_empty());

    for drive in drives {
        assert!(!drive.drive_id.is_empty());
        assert!(!drive.drive_name.is_empty());
        assert!(!drive.partition_name.is_empty());
        assert!(!drive.file_system.is_empty());
        assert!(drive.system_used <= drive.total_capacity);
    }
}
