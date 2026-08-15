use tauri::State;

use crate::app::state::AppState;
use crate::models::storage::DriveInfo;

#[tauri::command]
pub fn get_available_drives(state: State<'_, AppState>) -> Vec<DriveInfo> {
    state.storage.get_available_drives()
}
