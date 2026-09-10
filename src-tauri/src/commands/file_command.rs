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
pub async fn update_file_metadata(
    state: State<'_, AppState>,
    drive_id: String,
    path: String,
    name: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
    collection_ids: Option<Vec<String>>,
    favorite: Option<bool>,
    is_trashed: Option<bool>,
) -> AppResult<()> {
    let updates_labels =
        name.is_some() || category.is_some() || tags.is_some() || collection_ids.is_some();
    if !updates_labels && favorite.is_none() && is_trashed.is_none() {
        return Err(crate::error::AppError::validation(
            "No file metadata changes were provided.",
        ));
    }
    let (name, category, tags, collection_ids) = if updates_labels {
        let name = name
            .ok_or_else(|| crate::error::AppError::validation("A file name is required."))?
            .trim()
            .to_owned();
        if name.is_empty() || name.chars().count() > 255 || name.chars().any(char::is_control) {
            return Err(crate::error::AppError::validation(
                "File names must contain 1–255 characters without control characters.",
            ));
        }
        let category = category
            .ok_or_else(|| crate::error::AppError::validation("A category update is required."))?
            .trim()
            .to_owned();
        if category.chars().count() > 80 || category.chars().any(char::is_control) {
            return Err(crate::error::AppError::validation(
                "Categories must contain at most 80 characters without control characters.",
            ));
        }
        let tags = tags
            .ok_or_else(|| crate::error::AppError::validation("A tags update is required."))?
            .into_iter()
            .flat_map(|tag| {
                tag.split(|character: char| {
                    character.is_whitespace() || character == '_' || character == '-'
                })
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(str::to_lowercase)
                .collect::<Vec<_>>()
            })
            .fold(Vec::<String>::new(), |mut tags, tag| {
                if !tags
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&tag))
                {
                    tags.push(tag);
                }
                tags
            });
        if tags.len() > 100 {
            return Err(crate::error::AppError::validation(
                "A file can have at most 100 tags.",
            ));
        }
        if tags
            .iter()
            .any(|tag| tag.chars().count() > 80 || tag.chars().any(char::is_control))
        {
            return Err(crate::error::AppError::validation(
                "Tags must contain at most 80 characters without control characters.",
            ));
        }
        let collection_ids = collection_ids.ok_or_else(|| {
            crate::error::AppError::validation("A collections update is required.")
        })?;
        (Some(name), Some(category), Some(tags), Some(collection_ids))
    } else {
        (None, None, None, None)
    };
    let collection_names = if let Some(collection_ids) = collection_ids {
        Some(
            crate::repositories::collection_repository::selected_names(
                state.imports.collection_pool(),
                &collection_ids,
            )
            .await?,
        )
    } else {
        None
    };
    let _guard = state.imports.metadata_lock().lock().await;
    let (sidecar, file_id, current_name, current_collection_ids, labels_updated) = state
        .storage
        .update_file_metadata(
            drive_id.clone(),
            std::path::PathBuf::from(&path),
            name,
            category,
            tags,
            collection_names,
            favorite,
            is_trashed,
        )
        .await?;
    if labels_updated {
        crate::services::indexing_service::IndexingService::new(state.search.clone())
            .process_path(sidecar)
            .await?
    } else if let Some(favorite) = favorite {
        let index = state.search.clone();
        tauri::async_runtime::spawn_blocking(move || {
            index.index_filename(
                &drive_id,
                &file_id,
                &current_name,
                &current_collection_ids,
                favorite,
            )
        })
        .await
        .map_err(crate::error::AppError::internal)??;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_file_count(
    state: State<'_, AppState>,
    query: Option<String>,
    search_mode: Option<String>,
    tags: Option<Vec<String>>,
    collection: Option<String>,
    favorite_only: Option<bool>,
    trash_only: Option<bool>,
) -> AppResult<FileCountSummary> {
    let _guard = state.imports.metadata_lock().lock().await;
    state
        .storage
        .search_file_count(
            state.search.clone(),
            query.unwrap_or_default(),
            search_mode.unwrap_or_else(|| "tags".into()),
            tags.unwrap_or_default(),
            collection,
            favorite_only.unwrap_or(false),
            trash_only.unwrap_or(false),
        )
        .await
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
    collection: Option<String>,
    favorite_only: Option<bool>,
    trash_only: Option<bool>,
) -> AppResult<crate::models::file_model::FilePage> {
    let _guard = state.imports.metadata_lock().lock().await;
    let query = query.unwrap_or_default();
    let tags = tags
        .unwrap_or_default()
        .into_iter()
        .filter(|tag| !tag.trim().is_empty())
        .collect::<Vec<_>>();
    let page = {
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
                collection,
                favorite_only.unwrap_or(false),
                trash_only.unwrap_or(false),
            )
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

#[tauri::command]
pub async fn empty_trash(state: State<'_, AppState>) -> AppResult<()> {
    let _guard = state.imports.metadata_lock().lock().await;
    state.imports.empty_trash().await
}
