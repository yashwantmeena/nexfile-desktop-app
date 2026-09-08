use crate::utils::constants::COLLECTION_ID_LENGTH;
use crate::{
    error::{AppError, AppResult},
    models::collection_model::Collection,
};
use sqlx::SqlitePool;

pub async fn list(pool: &SqlitePool) -> AppResult<Vec<Collection>> {
    let rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
        "SELECT id, name, created_at_ms, updated_at_ms FROM collections ORDER BY name_key",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::database)?;
    Ok(rows
        .into_iter()
        .map(|(id, name, created_at_ms, updated_at_ms)| Collection {
            id,
            name,
            created_at_ms,
            updated_at_ms,
        })
        .collect())
}
pub async fn save(pool: &SqlitePool, id: Option<&str>, name: &str) -> AppResult<Vec<Collection>> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 60 || name.chars().any(char::is_control) {
        return Err(AppError::validation(
            "Collection names must contain 1–60 characters without control characters.",
        ));
    }
    let now = crate::utils::time::timestamp_ms()?;
    let key = name.to_lowercase();
    let mut transaction = pool.begin().await.map_err(AppError::database)?;
    let duplicate: Option<String> =
        sqlx::query_scalar("SELECT id FROM collections WHERE name_key = ?")
            .bind(&key)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(AppError::database)?;
    if duplicate.as_deref().is_some_and(|found| Some(found) != id) {
        return Err(AppError::validation(
            "A collection with this name already exists.",
        ));
    }
    if let Some(id) = id {
        let changed = sqlx::query("UPDATE collections SET updated_at_ms = CASE WHEN name != ? THEN MAX(updated_at_ms, ?) ELSE updated_at_ms END, name = ?, name_key = ? WHERE id = ?")
            .bind(name)
            .bind(now)
            .bind(name)
            .bind(key)
            .bind(id)
            .execute(&mut *transaction)
            .await
            .map_err(AppError::database)?;
        if changed.rows_affected() == 0 {
            return Err(AppError::validation("Collection not found."));
        }
    } else {
        loop {
            let id = nanoid::nanoid!(COLLECTION_ID_LENGTH);
            let inserted = sqlx::query(
                "INSERT OR IGNORE INTO collections (id, name, name_key, created_at_ms, updated_at_ms) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(name)
            .bind(&key).bind(now).bind(now)
            .execute(&mut *transaction)
            .await
            .map_err(AppError::database)?;
            if inserted.rows_affected() == 1 {
                break;
            }
        }
    }
    transaction.commit().await.map_err(AppError::database)?;
    list(pool).await
}
pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<Vec<Collection>> {
    sqlx::query("DELETE FROM collections WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::database)?;
    list(pool).await
}
pub async fn selected_names(pool: &SqlitePool, ids: &[String]) -> AppResult<Vec<String>> {
    if ids.len() > 100 {
        return Err(AppError::validation("Select at most 100 collections."));
    }
    let collections = list(pool).await?;
    let mut names = Vec::new();
    for id in ids {
        let collection = collections
            .iter()
            .find(|item| &item.id == id)
            .ok_or_else(|| AppError::validation("Selected collection no longer exists."))?;
        if !names.contains(&collection.name) {
            names.push(collection.name.clone());
        }
    }
    Ok(names)
}


