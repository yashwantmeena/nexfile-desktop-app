use tauri::State;

use crate::app::state::AppState;
use crate::error::AppResult;
use crate::models::background_process_model::BackgroundProcess;
use crate::models::import_model::ImportPreview;

#[tauri::command]
pub async fn preview_import(
    paths: Vec<String>,
    folder: bool,
    state: State<'_, AppState>,
) -> AppResult<ImportPreview> {
    state.imports.preview(paths, folder).await
}

#[tauri::command]
pub async fn import_file(
    paths: Vec<String>,
    collection_ids: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> AppResult<BackgroundProcess> {
    state
        .imports
        .import_with_collections(paths, false, collection_ids.unwrap_or_default())
        .await
}

#[tauri::command]
pub async fn import_folder(
    path: String,
    collection_ids: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> AppResult<BackgroundProcess> {
    state
        .imports
        .import_with_collections(vec![path], true, collection_ids.unwrap_or_default())
        .await
}

#[tauri::command]
pub async fn get_background_activities(
    state: State<'_, AppState>,
) -> AppResult<Vec<BackgroundProcess>> {
    state.imports.background_processes().list_active().await
}
