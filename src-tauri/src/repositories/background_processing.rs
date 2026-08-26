use super::database::SqliteDatabase;

pub struct SqliteBackgroundProcessingRepository {
    _database: SqliteDatabase,
}

impl SqliteBackgroundProcessingRepository {
    pub fn new(database: SqliteDatabase) -> Self {
        Self {
            _database: database,
        }
    }
}
