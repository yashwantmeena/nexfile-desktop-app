use tauri::State;

use crate::app::state::AppState;
use crate::error::AppResult;
use crate::models::file_model::FileCountSummary;

#[tauri::command]
pub async fn get_file_count(state: State<'_, AppState>) -> AppResult<FileCountSummary> {
    // Do not inspect between an import's SQLite commit and metadata-file write.
    let _guard = state.imports.metadata_lock().lock().await;
    state.storage.get_file_count().await
}
