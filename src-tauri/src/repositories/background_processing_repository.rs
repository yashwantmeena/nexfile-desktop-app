use crate::error::{AppError, AppResult};
use crate::models::background_process_model::BackgroundProcess;
use crate::types::background_process_status::BackgroundProcessStatus;
use crate::utils::operation_logger::log_event;

use super::database_repository::SqliteDatabase;

#[derive(Clone)]
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

    pub async fn acquire_import(
        &self,
        process_type: &str,
        count: u64,
        collections: &[String],
    ) -> AppResult<BackgroundProcess> {
        let mut collections = collections.to_vec();
        collections.sort_by_key(|name| name.to_lowercase());
        collections.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        let collections = serde_json::to_string(&collections).map_err(AppError::serialization)?;
        self.acquire(
            process_type,
            count,
            &collections,
            Some(&["import_file", "import_folder"]),
        )
        .await
    }

    pub async fn acquire_stage(
        &self,
        process_type: &str,
        count: u64,
    ) -> AppResult<BackgroundProcess> {
        self.acquire(process_type, count, "[]", None).await
    }

    async fn acquire(
        &self,
        process_type: &str,
        count: u64,
        collections: &str,
        compatible_types: Option<&[&str; 2]>,
    ) -> AppResult<BackgroundProcess> {
        let count = to_sql_integer(count)?;
        let mut transaction = self
            .database
            .pool()
            .begin()
            .await
            .map_err(AppError::database)?;
        let active = if compatible_types.is_some() {
            sqlx::query_as::<_, ProcessRow>(
                "SELECT process_id, process_type, status, priority, total_items, processed_items,
                        failed_items, remark, created_at_ms, updated_at_ms, started_at_ms, finished_at_ms
                 FROM background_processes
                 WHERE process_type IN ('import_file', 'import_folder')
                   AND collections = ?1 AND status IN ('queued', 'running')
                 ORDER BY created_at_ms LIMIT 1",
            )
            .bind(collections)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(AppError::database)?
        } else {
            sqlx::query_as::<_, ProcessRow>(
                "SELECT process_id, process_type, status, priority, total_items, processed_items,
                        failed_items, remark, created_at_ms, updated_at_ms, started_at_ms, finished_at_ms
                 FROM background_processes
                 WHERE process_type = ?1 AND status IN ('queued', 'running')
                 ORDER BY created_at_ms LIMIT 1",
            )
            .bind(process_type)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(AppError::database)?
        };

        let row = if let Some(active) = active {
            sqlx::query_as::<_, ProcessRow>(
                "UPDATE background_processes
                 SET total_items = total_items + ?2,
                     updated_at_ms = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
                 WHERE process_id = ?1
                 RETURNING process_id, process_type, status, priority, total_items, processed_items,
                           failed_items, remark, created_at_ms, updated_at_ms, started_at_ms, finished_at_ms",
            )
            .bind(&active.0)
            .bind(count)
            .fetch_one(&mut *transaction)
            .await
            .map_err(AppError::database)?
        } else {
            sqlx::query_as::<_, ProcessRow>(
                "INSERT INTO background_processes (
                    process_id, process_type, status, total_items, collections
                 ) VALUES (?1, ?2, 'queued', ?3, ?4)
                 RETURNING process_id, process_type, status, priority, total_items, processed_items,
                           failed_items, remark, created_at_ms, updated_at_ms, started_at_ms, finished_at_ms",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(process_type)
            .bind(count)
            .bind(collections)
            .fetch_one(&mut *transaction)
            .await
            .map_err(AppError::database)?
        };
        transaction.commit().await.map_err(AppError::database)?;
        let process = process_from_row(row)?;
        log_event(
            "background-process",
            "ACQUIRED",
            format!(
                "process_id={} process_type={} total_items={}",
                process.process_id, process.process_type, process.total_items
            ),
        );
        Ok(process)
    }

    pub async fn mark_running(&self, process_id: &str) -> AppResult<()> {
        let result = sqlx::query(
            "UPDATE background_processes
             SET status = 'running',
                 started_at_ms = COALESCE(started_at_ms,
                    CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)),
                 updated_at_ms = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
             WHERE process_id = ?1 AND status = 'queued'",
        )
        .bind(process_id)
        .execute(self.database.pool())
        .await
        .map_err(AppError::database)?;
        log_event(
            "background-process",
            "RUNNING",
            format!(
                "process_id={process_id} rows_affected={}",
                result.rows_affected()
            ),
        );
        Ok(())
    }

    pub async fn finish_item(&self, process_id: &str, failure: Option<&str>) -> AppResult<()> {
        let result = sqlx::query(
            "UPDATE background_processes
             SET processed_items = MIN(processed_items + 1, total_items),
                 failed_items = MIN(failed_items + CASE WHEN ?2 IS NULL THEN 0 ELSE 1 END, total_items),
                 remark = COALESCE(?2, remark),
                 status = CASE
                    WHEN processed_items + 1 >= total_items THEN
                        CASE WHEN failed_items + CASE WHEN ?2 IS NULL THEN 0 ELSE 1 END >= total_items
                            THEN 'failed' ELSE 'completed' END
                    ELSE 'running'
                 END,
                 finished_at_ms = CASE WHEN processed_items + 1 >= total_items
                    THEN CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
                    ELSE NULL END,
                 updated_at_ms = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
             WHERE process_id = ?1 AND status IN ('queued', 'running')",
        )
        .bind(process_id)
        .bind(failure)
        .execute(self.database.pool())
        .await
        .map_err(AppError::database)?;
        log_event(
            "background-process",
            if failure.is_some() {
                "ITEM-FAILED"
            } else {
                "ITEM-COMPLETE"
            },
            format!(
                "process_id={process_id} rows_affected={}",
                result.rows_affected()
            ),
        );
        Ok(())
    }

    pub async fn rollback_items(&self, process_id: &str, count: u64) -> AppResult<()> {
        let count = to_sql_integer(count)?;
        let mut transaction = self
            .database
            .pool()
            .begin()
            .await
            .map_err(AppError::database)?;
        sqlx::query(
            "UPDATE background_processes
             SET total_items = MAX(total_items - ?2, processed_items),
                 updated_at_ms = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
             WHERE process_id = ?1 AND status IN ('queued', 'running')",
        )
        .bind(process_id)
        .bind(count)
        .execute(&mut *transaction)
        .await
        .map_err(AppError::database)?;
        sqlx::query(
            "DELETE FROM background_processes
             WHERE process_id = ?1 AND total_items = 0 AND processed_items = 0",
        )
        .bind(process_id)
        .execute(&mut *transaction)
        .await
        .map_err(AppError::database)?;
        transaction.commit().await.map_err(AppError::database)?;
        log_event(
            "background-process",
            "ROLLBACK",
            format!("process_id={process_id} items={count}"),
        );
        Ok(())
    }

    pub async fn fail_remaining_items(
        &self,
        process_id: &str,
        count: u64,
        message: &str,
    ) -> AppResult<()> {
        let count = to_sql_integer(count)?;
        let result = sqlx::query(
            "UPDATE background_processes
             SET processed_items = MIN(processed_items + ?2, total_items),
                 failed_items = MIN(failed_items + ?2, total_items),
                 remark = ?3,
                 status = CASE
                    WHEN processed_items + ?2 >= total_items THEN
                        CASE WHEN failed_items + ?2 >= total_items
                            THEN 'failed' ELSE 'completed' END
                    ELSE 'running'
                 END,
                 finished_at_ms = CASE WHEN processed_items + ?2 >= total_items
                    THEN CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
                    ELSE NULL END,
                 updated_at_ms = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
             WHERE process_id = ?1 AND status IN ('queued', 'running')",
        )
        .bind(process_id)
        .bind(count)
        .bind(message)
        .execute(self.database.pool())
        .await
        .map_err(AppError::database)?;
        log_event(
            "background-process",
            "ITEMS-FAILED",
            format!(
                "process_id={process_id} items={count} rows_affected={}",
                result.rows_affected()
            ),
        );
        Ok(())
    }

    pub async fn list_active(&self) -> AppResult<Vec<BackgroundProcess>> {
        sqlx::query_as::<_, ProcessRow>(
            "SELECT process_id, process_type, status, priority, total_items, processed_items,
                    failed_items, remark, created_at_ms, updated_at_ms, started_at_ms, finished_at_ms
             FROM background_processes
             WHERE status IN ('queued', 'running')
             ORDER BY created_at_ms",
        )
        .fetch_all(self.database.pool())
        .await
        .map_err(AppError::database)?
        .into_iter()
        .map(process_from_row)
        .collect::<AppResult<Vec<_>>>()
    }

    pub async fn delete(&self, process_id: &str) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM background_processes WHERE process_id = ?1")
            .bind(process_id)
            .execute(self.database.pool())
            .await
            .map_err(AppError::database)?;
        log_event(
            "background-process",
            "DELETED",
            format!(
                "process_id={process_id} rows_affected={}",
                result.rows_affected()
            ),
        );
        Ok(result.rows_affected() > 0)
    }

    pub async fn close(&self) {
        self.database.close().await;
    }
}

type ProcessRow = (
    String,
    String,
    String,
    i64,
    i64,
    i64,
    i64,
    Option<String>,
    i64,
    i64,
    Option<i64>,
    Option<i64>,
);

fn process_from_row(row: ProcessRow) -> AppResult<BackgroundProcess> {
    Ok(BackgroundProcess {
        process_id: row.0,
        process_type: row.1,
        status: BackgroundProcessStatus::from_str(&row.2)
            .ok_or_else(|| AppError::validation("The background process has an invalid status."))?,
        priority: u32::try_from(row.3).map_err(AppError::internal)?,
        total_items: from_sql_integer(row.4)?,
        processed_items: from_sql_integer(row.5)?,
        failed_items: from_sql_integer(row.6)?,
        remark: row.7,
        created_at_ms: from_sql_integer(row.8)?,
        updated_at_ms: from_sql_integer(row.9)?,
        started_at_ms: row.10.map(from_sql_integer).transpose()?,
        finished_at_ms: row.11.map(from_sql_integer).transpose()?,
    })
}

fn to_sql_integer(value: u64) -> AppResult<i64> {
    i64::try_from(value).map_err(AppError::internal)
}

fn from_sql_integer(value: i64) -> AppResult<u64> {
    u64::try_from(value).map_err(AppError::internal)
}
