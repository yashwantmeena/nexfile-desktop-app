use tauri::State;

use crate::app::state::AppState;
use crate::error::AppResult;
use crate::models::storage::StorageData;

#[tauri::command]
pub fn get_storage_data(state: State<'_, AppState>) -> AppResult<StorageData> {
    state.storage.get_storage_data()
}
