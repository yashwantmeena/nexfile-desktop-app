use tauri::Manager;

use crate::error::AppResult;
use crate::repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
use crate::repositories::database_repository::SqliteDatabase;
use crate::repositories::indexing_repository::TantivyIndexingRepository;
use crate::repositories::storage_repository::SqliteStorageRepository;
use crate::services::import_service::ImportService;
use crate::services::indexing_service::IndexingService;
use crate::services::storage_service::StorageService;
use crate::services::trash_service::TrashService;
use crate::services::bulk_operation_service::BulkOperationService;
use crate::utils::constants::{
    AI_CONFIGS_DIRECTORY, AI_MODELS_DIRECTORY, CLIP_MODEL_DIRECTORY, FLORENCE2_MODEL_DIRECTORY,
};
use crate::utils::operation_logger::log_event;
use crate::workers::file_processing_worker::FileProcessingWorker;
use crate::workers::bulk_operation_worker::BulkOperationWorker;
use crate::workers::ai_processing_worker::AiProcessingWorker;
use crate::workers::indexing_worker::IndexingWorker;

use super::config::AppConfig;
use super::state::AppState;

pub fn initialize<R: tauri::Runtime>(app: &tauri::App<R>) -> AppResult<()> {
    log_event("app", "START", "application initialization started");
    let config = AppConfig::resolve(app)?;
    log_event(
        "app",
        "CONFIGURED",
        format!("app_data_dir={} resources_dir={}", config.app_data_dir.display(), config.resources_dir.display()),
    );
    std::fs::create_dir_all(&config.app_data_dir)?;
    let indexing_repository = TantivyIndexingRepository::open(&config.app_data_dir)?;
    log_event("app", "INDEX-OPEN", "search index opened");
    let system_metadata_root = config.app_data_dir.clone();
    let database = tauri::async_runtime::block_on(SqliteDatabase::open(config.database_path))?;
    log_event("app", "DATABASE-OPEN", "SQLite database opened");
    let storage_repository = SqliteStorageRepository::new(database.clone());
    let background_processing = SqliteBackgroundProcessingRepository::new(database.clone());
    let imports = tauri::async_runtime::block_on(ImportService::new(
        background_processing,
        SqliteStorageRepository::new(database.clone()),
        &database,
        system_metadata_root,
    ))?
    .with_search_index(indexing_repository.clone());
    log_event("app", "SERVICES-READY", "application services initialized");
    let indexing = IndexingService::new(indexing_repository.clone());
    let storage = StorageService::new(storage_repository, config.app_data_dir.clone()).with_drive_jobs(
        SqliteBackgroundProcessingRepository::new(database.clone()),
        &database,
        indexing_repository.clone(),
    );
    let trash = TrashService::new(
        SqliteBackgroundProcessingRepository::new(database.clone()),
        SqliteStorageRepository::new(database.clone()),
        &database,
        config.app_data_dir.clone(),
    );
    let bulk_operations = BulkOperationService::new(
        SqliteBackgroundProcessingRepository::new(database.clone()),
        SqliteStorageRepository::new(database.clone()),
        &database,
        config.app_data_dir.clone(),
        indexing_repository.clone(),
    );
    let file_processing_worker = FileProcessingWorker::start(
        &database,
        imports.clone(),
        storage.clone(),
        indexing_repository.clone(),
    );
    let bulk_operation_worker = BulkOperationWorker::start(
        &database,
        bulk_operations.clone(),
    );
    let ai_processing_worker = AiProcessingWorker::start(
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
    log_event("app", "WORKERS-STARTED", "background workers started");

    app.manage(AppState {
        search: indexing_repository,
        ai_processing_worker,
        file_processing_worker,
        bulk_operation_worker,
        indexing_worker,
        imports,
        storage,
        trash,
        bulk_operations,
    });
    log_event("app", "COMPLETE", "application initialization completed");
    Ok(())
}
