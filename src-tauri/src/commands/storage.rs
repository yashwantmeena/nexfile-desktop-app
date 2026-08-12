use tauri::State;

use crate::app::state::AppState;
use crate::error::AppResult;
use crate::models::storage::{
    SelectStorageTargetRequest, SetStorageAllocationRequest, StorageAllocation, StorageDevice,
    StorageTarget,
};

#[tauri::command]
pub fn get_available_storage(state: State<'_, AppState>) -> Vec<StorageDevice> {
    state.storage.available_devices()
}

#[tauri::command]
pub fn set_storage_allocation(
    state: State<'_, AppState>,
    request: SetStorageAllocationRequest,
) -> AppResult<Vec<StorageAllocation>> {
    state.storage.set_allocation(request)
}

#[tauri::command]
pub fn get_storage_allocations(state: State<'_, AppState>) -> AppResult<Vec<StorageAllocation>> {
    state.storage.allocations()
}

#[tauri::command]
pub fn remove_storage_allocation(
    state: State<'_, AppState>,
    volume_id: String,
) -> AppResult<Vec<StorageAllocation>> {
    state.storage.remove_allocation(&volume_id)
}

#[tauri::command]
pub fn clear_storage_allocations(state: State<'_, AppState>) -> AppResult<()> {
    state.storage.clear_allocations()
}

#[tauri::command]
pub fn select_storage_target(
    state: State<'_, AppState>,
    request: SelectStorageTargetRequest,
) -> AppResult<StorageTarget> {
    state.storage.select_target(request)
}
