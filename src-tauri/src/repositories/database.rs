use std::path::Path;
use std::sync::Arc;

use redb::Database;

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct RedbDatabase {
    inner: Arc<Database>,
}

impl RedbDatabase {
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        let database = Database::create(path).map_err(AppError::database)?;
        Ok(Self {
            inner: Arc::new(database),
        })
    }

    pub(crate) fn inner(&self) -> &Database {
        self.inner.as_ref()
    }
}
