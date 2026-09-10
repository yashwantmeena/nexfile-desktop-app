

use std::path::Path;

use crate::error::{AppError, AppResult};


use crate::models::storage_model::DriveMetadata;


use super::database_repository::SqliteDatabase;

#[derive(Clone)]
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
        let mut transaction = self
            .database
            .pool()
            .begin()
            .await
            .map_err(AppError::database)?;
        let saved = sqlx::query_as::<_, DriveMetadata>(
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
        .fetch_one(&mut *transaction)
        .await
        .map_err(AppError::database)?;

        transaction.commit().await.map_err(AppError::database)?;
        Ok(saved)
    }

    pub async fn update<F>(
        &self,
        drive: &DriveMetadata,
        persist_metadata: F,
    ) -> AppResult<DriveMetadata>
    where
        F: FnOnce(&DriveMetadata) -> AppResult<()>,
    {
        let mut transaction = self
            .database
            .pool()
            .begin()
            .await
            .map_err(AppError::database)?;
        // The drive UPDATE holds SQLite's write lock through the metadata write.
        let updated = update_drive(&mut transaction, drive).await?;
        if let Err(error) = persist_metadata(&updated) {
            transaction.rollback().await.map_err(AppError::database)?;
            return Err(error);
        }
        transaction.commit().await.map_err(AppError::database)?;
        Ok(updated)
    }

    pub async fn get(&self, drive_id: &str) -> AppResult<Option<DriveMetadata>> {
        Ok(self.fetch_drives(Some(drive_id)).await?.pop())
    }

    pub async fn list(&self) -> AppResult<Vec<DriveMetadata>> {
        self.fetch_drives(None).await
    }

    async fn fetch_drives(&self, drive_id: Option<&str>) -> AppResult<Vec<DriveMetadata>> {
        sqlx::query_as("SELECT * FROM drives WHERE (?1 IS NULL OR drive_id = ?1) ORDER BY drive_id")
            .bind(drive_id).fetch_all(self.database.pool()).await.map_err(AppError::database)
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
async fn update_drive(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    drive: &DriveMetadata,

) -> AppResult<DriveMetadata> {
    let saved = sqlx::query_as::<_, DriveMetadata>(
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
    .fetch_one(&mut **transaction)
    .await
    .map_err(AppError::database)?;



    Ok(saved)
}
