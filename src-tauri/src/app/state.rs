use crate::services::import_service::ImportService;
use crate::services::storage_service::StorageService;
use crate::services::trash_service::TrashService;
use crate::services::bulk_operation_service::BulkOperationService;
use crate::workers::file_processing_worker::FileProcessingWorker;
use crate::workers::bulk_operation_worker::BulkOperationWorker;
use crate::workers::ai_processing_worker::AiProcessingWorker;
use crate::workers::indexing_worker::IndexingWorker;

pub struct AppState {
    pub search: crate::repositories::indexing_repository::TantivyIndexingRepository,
    pub ai_processing_worker: AiProcessingWorker,
    pub file_processing_worker: FileProcessingWorker,
    pub bulk_operation_worker: BulkOperationWorker,
    pub indexing_worker: IndexingWorker,
    pub imports: ImportService,
    pub storage: StorageService,
    pub trash: TrashService,
    pub bulk_operations: BulkOperationService,
}
