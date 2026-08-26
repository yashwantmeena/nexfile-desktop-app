use std::path::Path;

use redb::{ReadableDatabase, ReadableTable, TableDefinition};

use crate::error::{AppError, AppResult};
use crate::models::storage::DriveMetadata;
use crate::utils::time::current_time_millis;

use super::database::RedbDatabase;

const DRIVES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("drives");

pub struct RedbStorageRepository {
    database: RedbDatabase,
}

impl RedbStorageRepository {
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        Self::new(RedbDatabase::open(path)?)
    }

    pub fn new(database: RedbDatabase) -> AppResult<Self> {
        let write = database.inner().begin_write().map_err(AppError::database)?;
        write.open_table(DRIVES_TABLE).map_err(AppError::database)?;
        write.commit().map_err(AppError::database)?;

        Ok(Self { database })
    }

    pub fn save(&self, drive: &DriveMetadata) -> AppResult<()> {
        self.replace(None, drive)
    }

    pub fn save_many(&self, drives: &[DriveMetadata]) -> AppResult<()> {
        let write = self
            .database
            .inner()
            .begin_write()
            .map_err(AppError::database)?;
        {
            let mut table = write.open_table(DRIVES_TABLE).map_err(AppError::database)?;
            for drive in drives {
                let existing = read_drive(&table, drive.drive_id.as_str())?;
                let timestamped = timestamped_drive(drive, existing.as_ref())?;
                let encoded = serde_json::to_vec(&timestamped).map_err(AppError::serialization)?;
                table
                    .insert(drive.drive_id.as_str(), encoded.as_slice())
                    .map_err(AppError::database)?;
            }
        }
        write.commit().map_err(AppError::database)
    }

    pub fn replace(&self, previous_drive_id: Option<&str>, drive: &DriveMetadata) -> AppResult<()> {
        let write = self
            .database
            .inner()
            .begin_write()
            .map_err(AppError::database)?;
        {
            let mut table = write.open_table(DRIVES_TABLE).map_err(AppError::database)?;
            let existing =
                read_drive(&table, previous_drive_id.unwrap_or(drive.drive_id.as_str()))?;
            let timestamped = timestamped_drive(drive, existing.as_ref())?;
            let encoded = serde_json::to_vec(&timestamped).map_err(AppError::serialization)?;
            if let Some(previous_drive_id) =
                previous_drive_id.filter(|previous| *previous != drive.drive_id)
            {
                table
                    .remove(previous_drive_id)
                    .map_err(AppError::database)?;
            }
            table
                .insert(drive.drive_id.as_str(), encoded.as_slice())
                .map_err(AppError::database)?;
        }
        write.commit().map_err(AppError::database)
    }

    pub fn list(&self) -> AppResult<Vec<DriveMetadata>> {
        let read = self
            .database
            .inner()
            .begin_read()
            .map_err(AppError::database)?;
        let table = read.open_table(DRIVES_TABLE).map_err(AppError::database)?;
        let mut drives = Vec::new();

        for entry in table.iter().map_err(AppError::database)? {
            let (_, value) = entry.map_err(AppError::database)?;
            drives.push(serde_json::from_slice(value.value()).map_err(AppError::serialization)?);
        }

        Ok(drives)
    }

    pub fn delete(&self, drive_id: &str) -> AppResult<bool> {
        let write = self
            .database
            .inner()
            .begin_write()
            .map_err(AppError::database)?;
        let removed = {
            let mut table = write.open_table(DRIVES_TABLE).map_err(AppError::database)?;
            let removed = table.remove(drive_id).map_err(AppError::database)?;
            removed.is_some()
        };
        write.commit().map_err(AppError::database)?;
        Ok(removed)
    }
}

fn read_drive(
    table: &impl ReadableTable<&'static str, &'static [u8]>,
    drive_id: &str,
) -> AppResult<Option<DriveMetadata>> {
    table
        .get(drive_id)
        .map_err(AppError::database)?
        .map(|value| serde_json::from_slice(value.value()).map_err(AppError::serialization))
        .transpose()
}

fn timestamped_drive(
    drive: &DriveMetadata,
    existing: Option<&DriveMetadata>,
) -> AppResult<DriveMetadata> {
    let now = current_time_millis()?;
    let mut timestamped = drive.clone();
    timestamped.created_at_ms = existing
        .map(|drive| drive.created_at_ms)
        .or_else(|| (drive.created_at_ms > 0).then_some(drive.created_at_ms))
        .unwrap_or(now);
    timestamped.updated_at_ms = now;
    Ok(timestamped)
}
