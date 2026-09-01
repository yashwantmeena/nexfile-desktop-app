pub mod ai_models;
mod app;
mod commands;
mod error;
mod mappers;
mod models;
mod repositories;
mod search;
mod services;
mod system;
pub mod utils;
mod workers;

use tauri::Manager;

pub use ai_models::clip::{ClipConfig, ClipModel, ClipModelPaths, Embedding};
pub use ai_models::florence2::{
    Florence2Config, Florence2Model, Florence2ModelPaths, Florence2Output, Florence2Task,
};
pub use error::{AppError, ClipError, Florence2Error};
pub use mappers::search_mapper::search_tags;
pub use models::background_process_model::{BackgroundProcess, BackgroundProcessStatus};
pub use models::image_processing_model::{
    ClassificationPrediction, ImageBoundingBox, ImageClassificationOutput, ImageLocation,
    ImageMetadata, ImageObjectDetection, ImageObjectDetectionOutput, ImageOcrOutput,
    ImageProcessingJob, ImageProcessingOutput,
};
pub use models::import_model::ImportFileJob;
pub use models::indexing_model::IndexingJob;
pub use models::storage_model::{
    DriveConfigurationUpdate, DriveInfo, DriveMetadata, StorageData, StorageDrive,
};
pub use repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
pub use repositories::database_repository::SqliteDatabase;
pub use repositories::storage_repository::SqliteStorageRepository;
pub use search::{SearchFields, SearchIndex};
pub use services::import_service::ImportService;
pub use services::indexing_service::IndexingService;
pub use services::storage_service::StorageService;
pub use system::filesystem::read_file;
pub use utils::image_hash::{calculate_phash, calculate_phash_path, format_phash, phash_distance};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app::lifecycle::initialize(app).map_err(|error| {
                // AppError intentionally hides infrastructure details from IPC
                // responses. Startup errors are local, so retain their source
                // chain in the development terminal for diagnosis.
                eprintln!("Failed to initialize NexFile: {error:?}");
                error.into()
            })
        })
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
            tauri::async_runtime::block_on(state.image_processing_worker.close());
            tauri::async_runtime::block_on(state.indexing_worker.close());
            tauri::async_runtime::block_on(state.imports.close());
            tauri::async_runtime::block_on(state.storage.close());
        }
    });
}
