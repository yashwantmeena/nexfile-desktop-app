use tauri::Manager;

use crate::error::AppResult;
use crate::repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
use crate::repositories::database_repository::SqliteDatabase;
use crate::repositories::storage_repository::SqliteStorageRepository;
use crate::search::SearchIndex;
use crate::services::import_service::ImportService;
use crate::services::storage_service::StorageService;
use crate::utils::constants::{
    AI_CONFIGS_DIRECTORY, AI_MODELS_DIRECTORY, CLIP_MODEL_DIRECTORY, FLORENCE2_MODEL_DIRECTORY,
};
use crate::workers::image_processing_worker::ImageProcessingWorker;
use crate::workers::import_worker::ImportWorker;

use super::config::AppConfig;
use super::state::AppState;

pub fn initialize<R: tauri::Runtime>(app: &tauri::App<R>) -> AppResult<()> {
    let config = AppConfig::resolve(app)?;
    std::fs::create_dir_all(&config.app_data_dir)?;
    let search = SearchIndex::open(&config.app_data_dir)?;
    let system_metadata_root = config.app_data_dir.clone();
    let database = tauri::async_runtime::block_on(SqliteDatabase::open(config.database_path))?;
    let storage_repository = SqliteStorageRepository::new(database.clone());
    let background_processing = SqliteBackgroundProcessingRepository::new(database.clone());
    let imports = tauri::async_runtime::block_on(ImportService::new(
        background_processing,
        SqliteStorageRepository::new(database.clone()),
        &database,
        system_metadata_root,
    ))?;
    let import_worker = ImportWorker::start(&database, imports.clone());
    let image_processing_worker = ImageProcessingWorker::start(
        &database,
        config
            .resources_dir
            .join(AI_MODELS_DIRECTORY)
            .join(CLIP_MODEL_DIRECTORY),
        config
            .resources_dir
            .join(AI_MODELS_DIRECTORY)
            .join(FLORENCE2_MODEL_DIRECTORY),
        config.resources_dir.join(AI_CONFIGS_DIRECTORY),
    );

    app.manage(AppState {
        image_processing_worker,
        import_worker,
        imports,
        search,
        storage: StorageService::new(storage_repository, config.app_data_dir),
    });
    Ok(())
}
