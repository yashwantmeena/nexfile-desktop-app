use crate::{
    app::state::AppState, error::AppResult, models::collection_model::Collection,
    repositories::collection_repository,
};
use tauri::State;

#[tauri::command]
pub async fn create_collection(
    state: State<'_, AppState>,
    name: String,
) -> AppResult<Vec<Collection>> {
    collection_repository::save(state.imports.collection_pool(), None, &name).await
}

#[tauri::command]
pub async fn list_collections(state: State<'_, AppState>) -> AppResult<Vec<Collection>> {
    collection_repository::list(state.imports.collection_pool()).await
}

#[tauri::command]
pub async fn update_collection(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> AppResult<Vec<Collection>> {
    collection_repository::save(state.imports.collection_pool(), Some(&id), &name).await
}

#[tauri::command]
pub async fn delete_collection(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Vec<Collection>> {
    collection_repository::delete(state.imports.collection_pool(), &id).await
}
