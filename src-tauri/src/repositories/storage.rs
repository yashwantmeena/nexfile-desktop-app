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
        let encoded = serde_json::to_vec(drive).map_err(AppError::serialization)?;
        let write = self.database.begin_write().map_err(AppError::database)?;
        {
            let mut table = write.open_table(DRIVES_TABLE).map_err(AppError::database)?;
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
}
