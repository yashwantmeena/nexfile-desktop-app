mod app;
mod commands;
mod error;
mod models;
mod repositories;
mod services;
mod system;

pub use error::AppError;
pub use models::storage::{DriveInfo, DriveMetadata, StorageData, StorageDrive};
pub use repositories::storage::RedbStorageRepository;
pub use services::storage_service::StorageService;
pub use system::filesystem::read_file;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| app::lifecycle::initialize(app).map_err(Into::into))
        .invoke_handler(tauri::generate_handler![
            commands::storage::get_storage_data,
            commands::storage::mount_drive,
            commands::storage::unmount_drive,
            commands::storage::remove_drive
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
