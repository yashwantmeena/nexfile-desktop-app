use crate::models::storage::DriveInfo;
use crate::system::filesystem::get_drives;

pub struct StorageService;

impl StorageService {
    pub fn get_available_drives(&self) -> Vec<DriveInfo> {
        get_drives()
    }
}
