use tauri::Manager;

use crate::error::AppResult;
use crate::repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
use crate::repositories::database_repository::SqliteDatabase;
use crate::repositories::indexing_repository::TantivyIndexingRepository;
use crate::repositories::storage_repository::SqliteStorageRepository;
use crate::services::import_service::ImportService;
use crate::services::indexing_service::IndexingService;
use crate::services::storage_service::StorageService;
use crate::utils::constants::{
    AI_CONFIGS_DIRECTORY, AI_MODELS_DIRECTORY, CLIP_MODEL_DIRECTORY, FLORENCE2_MODEL_DIRECTORY,
};
use crate::workers::image_processing_worker::ImageProcessingWorker;
use crate::workers::import_worker::ImportWorker;
use crate::workers::indexing_worker::IndexingWorker;
use crate::workers::delete_worker::DeleteWorker;

use super::config::AppConfig;
use super::state::AppState;

pub fn initialize<R: tauri::Runtime>(app: &tauri::App<R>) -> AppResult<()> {
    let config = AppConfig::resolve(app)?;
    std::fs::create_dir_all(&config.app_data_dir)?;
    let indexing_repository = TantivyIndexingRepository::open(&config.app_data_dir)?;
    let system_metadata_root = config.app_data_dir.clone();
    let database = tauri::async_runtime::block_on(SqliteDatabase::open(config.database_path))?;
    let storage_repository = SqliteStorageRepository::new(database.clone());
    let background_processing = SqliteBackgroundProcessingRepository::new(database.clone());
    let imports = tauri::async_runtime::block_on(ImportService::new(
        background_processing,
        SqliteStorageRepository::new(database.clone()),
        &database,
        system_metadata_root,
    ))?
    .with_search_index(indexing_repository.clone());
    let indexing = IndexingService::new(indexing_repository.clone());
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
        indexing.clone(),
    );
    let indexing_worker = IndexingWorker::start(&database, indexing);
    let storage = StorageService::new(storage_repository, config.app_data_dir);
    let delete_worker = DeleteWorker::start(&database, storage.clone(), indexing_repository.clone());

    app.manage(AppState {
        search: indexing_repository,
        image_processing_worker,
        import_worker,
        indexing_worker,
        delete_worker,
        imports,
        storage,
    });
    Ok(())
}
