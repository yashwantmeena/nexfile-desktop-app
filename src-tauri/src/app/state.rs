use crate::services::import_service::ImportService;
use crate::services::storage_service::StorageService;
use crate::workers::image_processing_worker::ImageProcessingWorker;
use crate::workers::import_worker::ImportWorker;
use crate::workers::indexing_worker::IndexingWorker;

pub struct AppState {
    pub image_processing_worker: ImageProcessingWorker,
    pub import_worker: ImportWorker,
    pub indexing_worker: IndexingWorker,
    pub imports: ImportService,
    pub storage: StorageService,
}
