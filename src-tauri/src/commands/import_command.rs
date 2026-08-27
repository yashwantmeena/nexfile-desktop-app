use tauri::State;

use crate::app::state::AppState;
use crate::error::AppResult;
use crate::models::background_process_model::BackgroundProcess;

#[tauri::command]
pub async fn import_file(
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> AppResult<BackgroundProcess> {
    state.imports.import_files(paths).await
}

#[tauri::command]
pub async fn import_folder(
    path: String,
    state: State<'_, AppState>,
) -> AppResult<BackgroundProcess> {
    state.imports.import_folder(path).await
}
