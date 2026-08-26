use tauri::Manager;

use crate::error::AppResult;
use crate::repositories::background_processing::RedbBackgroundProcessingRepository;
use crate::repositories::database::RedbDatabase;
use crate::repositories::storage::RedbStorageRepository;
use crate::services::storage_service::StorageService;

use super::config::AppConfig;
use super::state::AppState;

pub fn initialize<R: tauri::Runtime>(app: &tauri::App<R>) -> AppResult<()> {
    let config = AppConfig::resolve(app)?;
    std::fs::create_dir_all(&config.app_data_dir)?;
    let database = RedbDatabase::open(config.database_path)?;
    let storage_repository = RedbStorageRepository::new(database.clone())?;
    let background_processing = RedbBackgroundProcessingRepository::new(database)?;

    app.manage(background_processing);
    app.manage(AppState {
        storage: StorageService::new(storage_repository, config.app_data_dir),
    });
    Ok(())
}
