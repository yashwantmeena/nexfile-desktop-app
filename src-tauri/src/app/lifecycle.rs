use tauri::Manager;

use crate::services::storage_service::StorageService;

use super::state::AppState;

pub fn initialize<R: tauri::Runtime>(app: &tauri::App<R>) {
    app.manage(AppState {
        storage: StorageService,
    });
}
