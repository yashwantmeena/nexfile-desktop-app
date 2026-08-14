use std::path::Path;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

use crate::error::{AppError, AppResult};
use crate::models::storage::StorageAllocation;

const STORAGE_ALLOCATIONS_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("storage_allocations");

pub struct RedbStorageRepository {
    database: Database,
}

impl RedbStorageRepository {
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        let database = Database::create(path).map_err(AppError::database)?;
        let write = database.begin_write().map_err(AppError::database)?;
        write
            .open_table(STORAGE_ALLOCATIONS_TABLE)
            .map_err(AppError::database)?;
        write.commit().map_err(AppError::database)?;

        Ok(Self { database })
    }

    pub fn list(&self) -> AppResult<Vec<StorageAllocation>> {
        let read = self.database.begin_read().map_err(AppError::database)?;
        let table = read
            .open_table(STORAGE_ALLOCATIONS_TABLE)
            .map_err(AppError::database)?;
        let mut allocations = Vec::new();

        for entry in table.iter().map_err(AppError::database)? {
            let (_, value) = entry.map_err(AppError::database)?;
            allocations
                .push(serde_json::from_slice(value.value()).map_err(AppError::serialization)?);
        }

        Ok(allocations)
    }

    pub fn save_all(&self, allocations: &[StorageAllocation]) -> AppResult<()> {
        let write = self.database.begin_write().map_err(AppError::database)?;
        {
            let mut table = write
                .open_table(STORAGE_ALLOCATIONS_TABLE)
                .map_err(AppError::database)?;
            for allocation in allocations {
                let encoded = serde_json::to_vec(allocation).map_err(AppError::serialization)?;
                table
                    .insert(allocation.volume_id.as_str(), encoded.as_slice())
                    .map_err(AppError::database)?;
            }
        }
        write.commit().map_err(AppError::database)
    }

    pub fn remove_and_save(
        &self,
        volume_id: &str,
        allocations: &[StorageAllocation],
    ) -> AppResult<()> {
        let write = self.database.begin_write().map_err(AppError::database)?;
        {
            let mut table = write
                .open_table(STORAGE_ALLOCATIONS_TABLE)
                .map_err(AppError::database)?;
            table.remove(volume_id).map_err(AppError::database)?;
            for allocation in allocations {
                let encoded = serde_json::to_vec(allocation).map_err(AppError::serialization)?;
                table
                    .insert(allocation.volume_id.as_str(), encoded.as_slice())
                    .map_err(AppError::database)?;
            }
        }
        write.commit().map_err(AppError::database)
    }

    pub fn clear(&self) -> AppResult<()> {
        let write = self.database.begin_write().map_err(AppError::database)?;
        {
            let mut table = write
                .open_table(STORAGE_ALLOCATIONS_TABLE)
                .map_err(AppError::database)?;

            let mut volume_ids = Vec::new();
            for entry in table.iter().map_err(AppError::database)? {
                let (volume_id, _) = entry.map_err(AppError::database)?;
                volume_ids.push(volume_id.value().to_owned());
            }

            for volume_id in volume_ids {
                table
                    .remove(volume_id.as_str())
                    .map_err(AppError::database)?;
            }
        }
        write.commit().map_err(AppError::database)
    }
}
