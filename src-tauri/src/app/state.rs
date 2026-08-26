use crate::services::import_service::ImportService;
use crate::services::storage_service::StorageService;

pub struct AppState {
    pub imports: ImportService,
    pub storage: StorageService,
}
