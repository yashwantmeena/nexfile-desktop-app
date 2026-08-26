use redb::TableDefinition;

use crate::error::{AppError, AppResult};

use super::database::RedbDatabase;

const BACKGROUND_PROCESSING_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("background_processing");

pub struct RedbBackgroundProcessingRepository {
    _database: RedbDatabase,
}

impl RedbBackgroundProcessingRepository {
    pub fn new(database: RedbDatabase) -> AppResult<Self> {
        let write = database.inner().begin_write().map_err(AppError::database)?;
        write
            .open_table(BACKGROUND_PROCESSING_TABLE)
            .map_err(AppError::database)?;
        write.commit().map_err(AppError::database)?;
        Ok(Self {
            _database: database,
        })
    }
}
