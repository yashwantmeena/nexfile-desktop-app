use crate::error::{AppError, AppResult};
use crate::models::background_process::BackgroundProcess;

use super::database::SqliteDatabase;

pub struct SqliteBackgroundProcessingRepository {
    database: SqliteDatabase,
}

impl SqliteBackgroundProcessingRepository {
    pub fn new(database: SqliteDatabase) -> Self {
        Self { database }
    }

    pub async fn insert(&self, process: &BackgroundProcess) -> AppResult<BackgroundProcess> {
        let priority = i64::from(process.priority);
        let total_items = to_sql_integer(process.total_items)?;
        let processed_items = to_sql_integer(process.processed_items)?;
        let failed_items = to_sql_integer(process.failed_items)?;
        let started_at_ms = process.started_at_ms.map(to_sql_integer).transpose()?;
        let finished_at_ms = process.finished_at_ms.map(to_sql_integer).transpose()?;

        let (created_at_ms, updated_at_ms) = sqlx::query_as::<_, (i64, i64)>(
            "INSERT INTO background_processes (
                process_id,
                process_type,
                status,
                priority,
                total_items,
                processed_items,
                failed_items,
                remark,
                started_at_ms,
                finished_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            RETURNING created_at_ms, updated_at_ms",
        )
        .bind(&process.process_id)
        .bind(&process.process_type)
        .bind(process.status.as_str())
        .bind(priority)
        .bind(total_items)
        .bind(processed_items)
        .bind(failed_items)
        .bind(&process.remark)
        .bind(started_at_ms)
        .bind(finished_at_ms)
        .fetch_one(self.database.pool())
        .await
        .map_err(AppError::database)?;

        let mut saved = process.clone();
        saved.created_at_ms = from_sql_integer(created_at_ms)?;
        saved.updated_at_ms = from_sql_integer(updated_at_ms)?;
        Ok(saved)
    }

    pub async fn delete(&self, process_id: &str) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM background_processes WHERE process_id = ?1")
            .bind(process_id)
            .execute(self.database.pool())
            .await
            .map_err(AppError::database)?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn close(&self) {
        self.database.close().await;
    }
}

fn to_sql_integer(value: u64) -> AppResult<i64> {
    i64::try_from(value).map_err(AppError::internal)
}

fn from_sql_integer(value: i64) -> AppResult<u64> {
    u64::try_from(value).map_err(AppError::internal)
}
