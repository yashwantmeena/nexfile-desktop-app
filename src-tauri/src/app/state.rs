use crate::services::import_service::ImportService;
use crate::services::storage_service::StorageService;
use crate::services::trash_service::TrashService;
use crate::workers::image_processing_worker::ImageProcessingWorker;
use crate::workers::import_worker::ImportWorker;
use crate::workers::indexing_worker::IndexingWorker;
use crate::workers::delete_worker::DeleteWorker;

pub struct AppState {
    pub search: crate::repositories::indexing_repository::TantivyIndexingRepository,
    pub image_processing_worker: ImageProcessingWorker,
    pub import_worker: ImportWorker,
    pub indexing_worker: IndexingWorker,
    pub delete_worker: DeleteWorker,
    pub imports: ImportService,
    pub storage: StorageService,
    pub trash: TrashService,
}
