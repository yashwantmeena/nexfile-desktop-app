use crate::{
    app::state::AppState, error::AppResult, models::collection_model::Collection,
    repositories::collection_repository,
};
use tauri::State;
use crate::utils::operation_logger::log_event;

#[tauri::command]
pub async fn create_collection(
    state: State<'_, AppState>,
    name: String,
) -> AppResult<Vec<Collection>> {
    log_event("collections", "CREATE-START", format!("name_length={}", name.chars().count()));
    let result = collection_repository::save(state.imports.collection_pool(), None, &name).await;
    log_event(
        "collections",
        if result.is_ok() { "CREATE-COMPLETE" } else { "CREATE-FAILED" },
        format!("name_length={} result={}", name.chars().count(), result.is_ok()),
    );
    result
}

#[tauri::command]
pub async fn list_collections(state: State<'_, AppState>) -> AppResult<Vec<Collection>> {
    log_event("collections", "LIST-START", "listing catalog collections");
    let result = collection_repository::list(state.imports.collection_pool()).await;
    match &result {
        Ok(collections) => log_event("collections", "LIST-COMPLETE", format!("count={}", collections.len())),
        Err(error) => log_event("collections", "LIST-FAILED", format!("error={error}")),
    }
    result
}

#[tauri::command]
pub async fn update_collection(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> AppResult<Vec<Collection>> {
    log_event("collections", "UPDATE-START", format!("id={id} name_length={}", name.chars().count()));
    let result = collection_repository::save(state.imports.collection_pool(), Some(&id), &name).await;
    log_event(
        "collections",
        if result.is_ok() { "UPDATE-COMPLETE" } else { "UPDATE-FAILED" },
        format!("id={id} result={}", result.is_ok()),
    );
    result
}

#[tauri::command]
pub async fn delete_collection(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Vec<Collection>> {
    log_event("collections", "DELETE-START", format!("id={id}"));
    let result = collection_repository::delete(state.imports.collection_pool(), &id).await;
    log_event(
        "collections",
        if result.is_ok() { "DELETE-COMPLETE" } else { "DELETE-FAILED" },
        format!("id={id} result={}", result.is_ok()),
    );
    result
}
