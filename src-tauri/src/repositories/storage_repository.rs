use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::models::storage_model::DriveMetadata;

use super::database_repository::SqliteDatabase;

pub struct SqliteStorageRepository {
    database: SqliteDatabase,
}

impl SqliteStorageRepository {
    pub async fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        Ok(Self::new(SqliteDatabase::open(path).await?))
    }

    pub fn new(database: SqliteDatabase) -> Self {
        Self { database }
    }

    pub async fn insert(&self, drive: &DriveMetadata) -> AppResult<DriveMetadata> {
        sqlx::query_as::<_, DriveMetadata>(
            "INSERT INTO drives (
                drive_id,
                drive_name,
                partition_name,
                app_limit_bytes,
                file_count,
                app_used_bytes,
                priority,
                is_mounted
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            RETURNING *",
        )
        .bind(&drive.drive_id)
        .bind(&drive.drive_name)
        .bind(&drive.partition_name)
        .bind(drive.app_limit_bytes)
        .bind(drive.file_count)
        .bind(drive.app_used_bytes)
        .bind(drive.priority)
        .bind(drive.is_mounted)
        .fetch_one(self.database.pool())
        .await
        .map_err(AppError::database)
    }

    pub async fn update(&self, drive: &DriveMetadata) -> AppResult<DriveMetadata> {
        sqlx::query_as::<_, DriveMetadata>(
            "UPDATE drives SET
                drive_name = ?1,
                partition_name = ?2,
                app_limit_bytes = ?3,
                file_count = ?4,
                app_used_bytes = ?5,
                priority = ?6,
                is_mounted = ?7,
                updated_at_ms = CAST(
                    (julianday('now') - 2440587.5) * 86400000 AS INTEGER
                )
            WHERE drive_id = ?8
            RETURNING *",
        )
        .bind(&drive.drive_name)
        .bind(&drive.partition_name)
        .bind(drive.app_limit_bytes)
        .bind(drive.file_count)
        .bind(drive.app_used_bytes)
        .bind(drive.priority)
        .bind(drive.is_mounted)
        .bind(&drive.drive_id)
        .fetch_one(self.database.pool())
        .await
        .map_err(AppError::database)
    }

    pub async fn get(&self, drive_id: &str) -> AppResult<Option<DriveMetadata>> {
        sqlx::query_as::<_, DriveMetadata>("SELECT * FROM drives WHERE drive_id = ?1")
            .bind(drive_id)
            .fetch_optional(self.database.pool())
            .await
            .map_err(AppError::database)
    }

    pub async fn list(&self) -> AppResult<Vec<DriveMetadata>> {
        sqlx::query_as::<_, DriveMetadata>("SELECT * FROM drives ORDER BY drive_id")
            .fetch_all(self.database.pool())
            .await
            .map_err(AppError::database)
    }

    pub async fn delete(&self, drive_id: &str) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM drives WHERE drive_id = ?1")
            .bind(drive_id)
            .execute(self.database.pool())
            .await
            .map_err(AppError::database)?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn close(&self) {
        self.database.close().await;
    }
}
