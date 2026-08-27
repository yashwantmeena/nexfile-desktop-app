pub mod ai_models;
mod app;
mod commands;
mod error;
mod mappers;
mod models;
mod repositories;
mod services;
mod system;
pub mod utils;
mod workers;

use tauri::Manager;

pub use ai_models::clip::{ClipConfig, ClipError, ClipModel, ClipModelPaths, Embedding};
pub use ai_models::florence2::{
    Florence2Config, Florence2Error, Florence2Model, Florence2ModelPaths, Florence2Output,
    Florence2Task,
};
pub use error::AppError;
pub use models::background_process_model::{BackgroundProcess, BackgroundProcessStatus};
pub use models::import_model::ImportFileJob;
pub use models::storage_model::{
    DriveConfigurationUpdate, DriveInfo, DriveMetadata, StorageData, StorageDrive,
};
pub use repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
pub use repositories::database_repository::SqliteDatabase;
pub use repositories::storage_repository::SqliteStorageRepository;
pub use services::import_service::ImportService;
pub use services::storage_service::StorageService;
pub use system::filesystem::read_file;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| app::lifecycle::initialize(app).map_err(Into::into))
        .invoke_handler(tauri::generate_handler![
            commands::import_command::import_file,
            commands::import_command::import_folder,
            commands::storage_command::get_storage_data,
            commands::storage_command::mount_drive,
            commands::storage_command::unmount_drive,
            commands::storage_command::update_drive_configuration,
            commands::storage_command::remove_drive
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            let state = app_handle.state::<app::state::AppState>();
            tauri::async_runtime::block_on(state.import_worker.close());
            tauri::async_runtime::block_on(state.imports.close());
            tauri::async_runtime::block_on(state.storage.close());
        }
    });
}
