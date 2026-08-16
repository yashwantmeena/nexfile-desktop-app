use std::path::Path;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

use crate::error::{AppError, AppResult};
use crate::models::storage::DriveMetadata;

const DRIVES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("drives");

pub struct RedbStorageRepository {
    database: Database,
}

impl RedbStorageRepository {
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        let database = Database::create(path).map_err(AppError::database)?;
        let write = database.begin_write().map_err(AppError::database)?;
        write.open_table(DRIVES_TABLE).map_err(AppError::database)?;
        write.commit().map_err(AppError::database)?;

        Ok(Self { database })
    }

    pub fn save(&self, drive: &DriveMetadata) -> AppResult<()> {
        self.replace(None, drive)
    }

    pub fn save_many(&self, drives: &[DriveMetadata]) -> AppResult<()> {
        let encoded_drives = drives
            .iter()
            .map(|drive| {
                serde_json::to_vec(drive)
                    .map(|encoded| (drive.drive_id.as_str(), encoded))
                    .map_err(AppError::serialization)
            })
            .collect::<AppResult<Vec<_>>>()?;

        let write = self.database.begin_write().map_err(AppError::database)?;
        {
            let mut table = write.open_table(DRIVES_TABLE).map_err(AppError::database)?;
            for (drive_id, encoded) in encoded_drives {
                table
                    .insert(drive_id, encoded.as_slice())
                    .map_err(AppError::database)?;
            }
        }
        write.commit().map_err(AppError::database)
    }

    pub fn replace(&self, previous_drive_id: Option<&str>, drive: &DriveMetadata) -> AppResult<()> {
        let encoded = serde_json::to_vec(drive).map_err(AppError::serialization)?;
        let write = self.database.begin_write().map_err(AppError::database)?;
        {
            let mut table = write.open_table(DRIVES_TABLE).map_err(AppError::database)?;
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
        let read = self.database.begin_read().map_err(AppError::database)?;
        let table = read.open_table(DRIVES_TABLE).map_err(AppError::database)?;
        let mut drives = Vec::new();

        for entry in table.iter().map_err(AppError::database)? {
            let (_, value) = entry.map_err(AppError::database)?;
            drives.push(serde_json::from_slice(value.value()).map_err(AppError::serialization)?);
        }

        Ok(drives)
    }

    pub fn delete(&self, drive_id: &str) -> AppResult<bool> {
        let write = self.database.begin_write().map_err(AppError::database)?;
        let removed = {
            let mut table = write.open_table(DRIVES_TABLE).map_err(AppError::database)?;
            let removed = table.remove(drive_id).map_err(AppError::database)?;
            removed.is_some()
        };
        write.commit().map_err(AppError::database)?;
        Ok(removed)
    }
}
