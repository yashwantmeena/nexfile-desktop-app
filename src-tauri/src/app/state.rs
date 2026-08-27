use crate::services::import_service::ImportService;
use crate::services::storage_service::StorageService;
use crate::workers::import_worker::ImportWorker;

pub struct AppState {
    pub import_worker: ImportWorker,
    pub imports: ImportService,
    pub storage: StorageService,
}
