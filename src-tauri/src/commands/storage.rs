use tauri::State;

use crate::app::state::AppState;
use crate::error::AppResult;
use crate::models::storage::{DriveConfigurationUpdate, StorageData};

#[tauri::command]
pub async fn get_storage_data(state: State<'_, AppState>) -> AppResult<StorageData> {
    state.storage.get_storage_data().await
}

#[tauri::command]
pub async fn mount_drive(
    device_id: Option<String>,
    partition_name: String,
    state: State<'_, AppState>,
) -> AppResult<StorageData> {
    state
        .storage
        .mount_drive(device_id.as_deref(), &partition_name)
        .await
}

#[tauri::command]
pub async fn unmount_drive(drive_id: String, state: State<'_, AppState>) -> AppResult<StorageData> {
    state.storage.unmount_drive(&drive_id).await
}

#[tauri::command]
pub async fn update_drive_configuration(
    drives: Vec<DriveConfigurationUpdate>,
    state: State<'_, AppState>,
) -> AppResult<StorageData> {
    state.storage.update_drive_configuration(&drives).await
}

#[tauri::command]
pub async fn remove_drive(drive_id: String, state: State<'_, AppState>) -> AppResult<StorageData> {
    state.storage.remove_drive(&drive_id).await
}
