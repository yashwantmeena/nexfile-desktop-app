mod app;
mod commands;
mod error;
mod models;
mod services;
mod system;

pub use error::AppError;
pub use models::storage::DriveInfo;
pub use services::storage_service::StorageService;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app::lifecycle::initialize(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::storage::get_available_drives
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
