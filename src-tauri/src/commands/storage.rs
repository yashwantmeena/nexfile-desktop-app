use tauri::State;

use crate::app::state::AppState;
use crate::error::AppResult;
use crate::models::storage::StorageData;

#[tauri::command]
pub fn get_storage_data(state: State<'_, AppState>) -> AppResult<StorageData> {
    state.storage.get_storage_data()
}

#[tauri::command]
pub fn mount_drive(
    device_id: Option<String>,
    partition_name: String,
    state: State<'_, AppState>,
) -> AppResult<StorageData> {
    state
        .storage
        .mount_drive(device_id.as_deref(), &partition_name)
}

#[tauri::command]
pub fn unmount_drive(drive_id: String, state: State<'_, AppState>) -> AppResult<StorageData> {
    state.storage.unmount_drive(&drive_id)
}

#[tauri::command]
pub fn remove_drive(drive_id: String, state: State<'_, AppState>) -> AppResult<StorageData> {
    state.storage.remove_drive(&drive_id)
}
