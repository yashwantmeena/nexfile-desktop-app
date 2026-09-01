use crate::search::SearchIndex;
use crate::services::import_service::ImportService;
use crate::services::storage_service::StorageService;
use crate::workers::image_processing_worker::ImageProcessingWorker;
use crate::workers::import_worker::ImportWorker;

pub struct AppState {
    pub image_processing_worker: ImageProcessingWorker,
    pub import_worker: ImportWorker,
    pub imports: ImportService,
    pub search: SearchIndex,
    pub storage: StorageService,
}
