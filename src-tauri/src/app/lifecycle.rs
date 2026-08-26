use tauri::Manager;

use crate::error::AppResult;
use crate::repositories::background_processing::SqliteBackgroundProcessingRepository;
use crate::repositories::database::SqliteDatabase;
use crate::repositories::storage::SqliteStorageRepository;
use crate::services::storage_service::StorageService;

use super::config::AppConfig;
use super::state::AppState;

pub fn initialize<R: tauri::Runtime>(app: &tauri::App<R>) -> AppResult<()> {
    let config = AppConfig::resolve(app)?;
    std::fs::create_dir_all(&config.app_data_dir)?;
    let database = tauri::async_runtime::block_on(SqliteDatabase::open(config.database_path))?;
    let storage_repository = SqliteStorageRepository::new(database.clone());
    let background_processing = SqliteBackgroundProcessingRepository::new(database);

    app.manage(background_processing);
    app.manage(AppState {
        storage: StorageService::new(storage_repository, config.app_data_dir),
    });
    Ok(())
}
