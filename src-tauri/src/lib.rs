mod app;
mod commands;
mod error;
mod models;
mod repositories;
mod services;
mod system;
mod utils;

pub use error::AppError;
pub use models::storage::{StorageAllocation, StorageDevice, StorageKind, StorageTarget};
pub use repositories::storage::RedbStorageRepository;
pub use services::storage_service::choose_storage_target;
pub use system::filesystem::{detect_storage_devices, map_disk_kind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| app::lifecycle::initialize(app).map_err(Into::into))
        .invoke_handler(tauri::generate_handler![
            commands::storage::get_available_storage,
            commands::storage::set_storage_allocation,
            commands::storage::get_storage_allocations,
            commands::storage::remove_storage_allocation,
            commands::storage::clear_storage_allocations,
            commands::storage::select_storage_target
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
