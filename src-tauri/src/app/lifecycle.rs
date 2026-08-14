use tauri::Manager;

use crate::error::AppResult;
use crate::repositories::storage::RedbStorageRepository;
use crate::services::storage_service::StorageService;

use super::config::AppConfig;
use super::state::AppState;

pub fn initialize<R: tauri::Runtime>(app: &tauri::App<R>) -> AppResult<()> {
    let config = AppConfig::resolve(app)?;
    std::fs::create_dir_all(&config.app_data_dir)?;
    let repository = RedbStorageRepository::open(config.database_path)?;
    app.manage(AppState {
        storage: StorageService::new(repository),
    });
    Ok(())
}
