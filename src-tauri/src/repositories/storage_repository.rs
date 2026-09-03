use sqlx::{FromRow, Row};
use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::mappers::file_mapper::normalize_counts;
use crate::models::file_model::FileTypeCount;
use crate::models::storage_model::DriveMetadata;
use crate::types::file_type::FileType;

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
        let mut transaction = self
            .database
            .pool()
            .begin()
            .await
            .map_err(AppError::database)?;
        let mut saved = sqlx::query_as::<_, DriveMetadata>(
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
        saved.file_type_counts =
            normalize_counts(drive.file_type_counts.clone()).map_err(AppError::validation)?;
        persist_counts(&mut transaction, &saved).await?;
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
        let counts =
            normalize_counts(drive.file_type_counts.clone()).map_err(AppError::validation)?;
        let total = counts
            .iter()
            .try_fold(0_i64, |sum, entry| sum.checked_add(entry.count));
        if total != Some(drive.file_count) {
            return Err(AppError::validation(
                "Category counts must equal the drive's file count.",
            ));
        }
        let mut transaction = self
            .database
            .pool()
            .begin()
            .await
            .map_err(AppError::database)?;
        // The drive UPDATE holds SQLite's write lock through the metadata write.
        let updated = update_drive(&mut transaction, drive, counts).await?;
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

    // One joined query gives drive totals and category counts from the same snapshot.
    async fn fetch_drives(&self, drive_id: Option<&str>) -> AppResult<Vec<DriveMetadata>> {
        let mut query = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
            "SELECT drives.*, counts.file_type AS count_file_type, counts.count AS type_count
             FROM drives LEFT JOIN drive_file_type_counts AS counts
             ON counts.drive_id = drives.drive_id",
        );
        if let Some(drive_id) = drive_id {
            query.push(" WHERE drives.drive_id = ").push_bind(drive_id);
        }
        let rows = query
            .build()
            .fetch_all(self.database.pool())
            .await
            .map_err(AppError::database)?;
        let mut drives = BTreeMap::<String, DriveMetadata>::new();
        for row in rows {
            let id: String = row.try_get("drive_id").map_err(AppError::database)?;
            let drive = match drives.entry(id) {
                std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(DriveMetadata::from_row(&row).map_err(AppError::database)?)
                }
            };
            if let Some(file_type) = row
                .try_get::<Option<FileType>, _>("count_file_type")
                .map_err(AppError::database)?
            {
                drive.file_type_counts.push(FileTypeCount {
                    file_type,
                    count: row.try_get("type_count").map_err(AppError::database)?,
                });
            }
        }
        drives
            .into_values()
            .map(|mut drive| {
                drive.file_type_counts =
                    normalize_counts(drive.file_type_counts).map_err(AppError::validation)?;
                Ok(drive)
            })
            .collect()
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
async fn persist_counts(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    drive: &DriveMetadata,
) -> AppResult<()> {
    if drive.file_type_counts.is_empty() {
        return Ok(());
    }
    let mut query = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
        "INSERT INTO drive_file_type_counts (drive_id, file_type, count) ",
    );
    query.push_values(&drive.file_type_counts, |mut row, entry| {
        row.push_bind(&drive.drive_id)
            .push_bind(entry.file_type.as_str())
            .push_bind(entry.count);
    });
    query.push(
        " ON CONFLICT(drive_id, file_type) DO UPDATE SET
            count = excluded.count,
            updated_at_ms = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
         WHERE drive_file_type_counts.count != excluded.count",
    );
    query
        .build()
        .execute(&mut **transaction)
        .await
        .map_err(AppError::database)?;
    Ok(())
}

async fn update_drive(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    drive: &DriveMetadata,
    counts: Vec<FileTypeCount>,
) -> AppResult<DriveMetadata> {
    let mut saved = sqlx::query_as::<_, DriveMetadata>(
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

    saved.file_type_counts = counts;
    persist_counts(transaction, &saved).await?;
    Ok(saved)
}
