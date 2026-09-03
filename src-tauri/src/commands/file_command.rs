use tauri::{Manager, State};

use crate::app::state::AppState;
use crate::error::AppResult;
use crate::models::file_model::FileCountSummary;

#[tauri::command]
pub async fn get_file_count(state: State<'_, AppState>) -> AppResult<FileCountSummary> {
    // Do not inspect between an import's SQLite commit and metadata-file write.
    let _guard = state.imports.metadata_lock().lock().await;
    state.storage.get_file_count().await
}

#[tauri::command]
pub async fn fetch_files(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> AppResult<crate::models::file_model::FilePage> {
    let _guard = state.imports.metadata_lock().lock().await;
    let page = state
        .storage
        .fetch_files(offset.unwrap_or(0), limit.unwrap_or(60))
        .await?;
    // Grant access only to the image files returned in this page.
    for file in &page.files {
        if matches!(
            file.file_type,
            crate::types::file_type::FileType::Image | crate::types::file_type::FileType::Video
        ) {
            app.asset_protocol_scope()
                .allow_file(&file.path)
                .map_err(crate::error::AppError::internal)?;
        }
    }
    Ok(page)
}
