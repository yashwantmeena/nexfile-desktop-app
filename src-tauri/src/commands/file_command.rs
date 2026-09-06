use tauri::{Manager, State};

use crate::app::state::AppState;
use crate::error::AppResult;
use crate::models::file_model::FileCountSummary;

#[tauri::command]
pub async fn suggest_tags(state: State<'_, AppState>, prefix: String) -> AppResult<Vec<String>> {
    let repository = state.search.clone();
    tauri::async_runtime::spawn_blocking(move || repository.suggest_tags(&prefix))
        .await
        .map_err(crate::error::AppError::internal)?
}

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
    media_type: Option<crate::types::file_type::FileType>,
    query: Option<String>,
    search_mode: Option<String>,
    tags: Option<Vec<String>>,
) -> AppResult<crate::models::file_model::FilePage> {
    let _guard = state.imports.metadata_lock().lock().await;
    let query = query.unwrap_or_default();
    let tags = tags
        .unwrap_or_default()
        .into_iter()
        .filter(|tag| !tag.trim().is_empty())
        .collect::<Vec<_>>();
    let page = if !query.trim().is_empty() || !tags.is_empty() {
        state
            .storage
            .search_files(
                state.search.clone(),
                query,
                search_mode.unwrap_or_else(|| "tags".to_owned()),
                tags,
                offset.unwrap_or(0),
                limit.unwrap_or(60),
                media_type,
            )
            .await?
    } else {
        state
            .storage
            .fetch_files(offset.unwrap_or(0), limit.unwrap_or(60), media_type)
            .await?
    };
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
