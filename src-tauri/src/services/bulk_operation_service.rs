use std::collections::HashSet;
use std::path::PathBuf;

use apalis::prelude::*;
use apalis_sqlite::SqliteStorage;
use futures::stream;

use crate::error::{AppError, AppResult};
use crate::models::background_process_model::BackgroundProcess;
use crate::models::bulk_operation_model::{
    BulkFileTarget, BulkOperation, BulkOperationFilters, BulkOperationJob,
};
use crate::repositories::background_processing_repository::SqliteBackgroundProcessingRepository;
use crate::repositories::database_repository::SqliteDatabase;
use crate::repositories::indexing_repository::TantivyIndexingRepository;
use crate::repositories::storage_repository::SqliteStorageRepository;
use crate::system::filesystem::get_drives;
use crate::types::background_process_status::BackgroundProcessStatus;
use crate::utils::constants::{BULK_OPERATION_PROCESS_TYPE, BULK_OPERATION_QUEUE};
use crate::utils::operation_logger::log_event;

#[derive(Clone)]
pub struct BulkOperationService {
    repository: SqliteBackgroundProcessingRepository,
    queue_pool: sqlx::SqlitePool,
    storage_repository: SqliteStorageRepository,
    system_metadata_root: PathBuf,
    index: TantivyIndexingRepository,
}

impl BulkOperationService {
    pub fn new(
        repository: SqliteBackgroundProcessingRepository,
        storage_repository: SqliteStorageRepository,
        database: &SqliteDatabase,
        system_metadata_root: PathBuf,
        index: TantivyIndexingRepository,
    ) -> Self {
        Self {
            repository,
            queue_pool: database.pool().clone(),
            storage_repository,
            system_metadata_root,
            index,
        }
    }

    pub async fn enqueue(
        &self,
        operation: BulkOperation,
        filters: BulkOperationFilters,
        selected_ids: Option<Vec<String>>,
        expected_count: u64,
    ) -> AppResult<BackgroundProcess> {
        if expected_count == 0 {
            return Err(AppError::validation("No files are selected."));
        }
        let snapshot_at_ms = current_time_ms()?;
        log_event(
            BULK_OPERATION_QUEUE,
            "REQUEST",
            format!(
                "operation={} snapshot_at_ms={} selected_ids={}",
                operation.as_str(),
                snapshot_at_ms,
                selected_ids.as_ref().map_or(0, Vec::len)
            ),
        );
        let process = self
            .repository
            .insert(&BackgroundProcess {
                process_id: uuid::Uuid::new_v4().to_string(),
                process_type: BULK_OPERATION_PROCESS_TYPE.to_owned(),
                status: BackgroundProcessStatus::Queued,
                priority: 0,
                total_items: expected_count,
                processed_items: 0,
                failed_items: 0,
                remark: None,
                created_at_ms: 0,
                updated_at_ms: 0,
                started_at_ms: None,
                finished_at_ms: None,
            })
            .await?;
        let job = BulkOperationJob {
            process_id: process.process_id.clone(),
            expected_count,
            operation,
            filters,
            snapshot_at_ms,
            selected_ids,
        };
        let mut queue = SqliteStorage::<BulkOperationJob, (), ()>::new_in_queue(
            &self.queue_pool,
            BULK_OPERATION_QUEUE,
        );
        let mut jobs = stream::iter([Task::builder(job).build()]);
        if let Err(error) = queue.push_all(&mut jobs).await {
            let _ = self
                .repository
                .rollback_items(&process.process_id, expected_count)
                .await;
            return Err(AppError::database(error));
        }
        log_event(
            BULK_OPERATION_QUEUE,
            "QUEUED",
            format!(
                "process_id={} operation={} items={} snapshot_at_ms={snapshot_at_ms}",
                process.process_id,
                process.process_type,
                expected_count
            ),
        );
        Ok(process)
    }

    pub(crate) async fn collect_targets(
        &self,
        filters: BulkOperationFilters,
        snapshot_at_ms: i64,
        selected_ids: Option<Vec<String>>,
    ) -> AppResult<Vec<BulkFileTarget>> {
        log_event(
            BULK_OPERATION_QUEUE,
            "MATCH-START",
            format!(
                "snapshot_at_ms={snapshot_at_ms} selected_ids={} query_length={}",
                selected_ids.as_ref().map_or(0, Vec::len),
                filters.query.chars().count()
            ),
        );
        let snapshots = self.storage_repository.list().await?;
        let root = self.system_metadata_root.clone();
        let index = self.index.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            let connected = get_drives();
            let page = if filters.trash_only {
                crate::services::file_service::fetch_matching_files_with_trash(
                    snapshots,
                    connected,
                    &root,
                    filters.media_type,
                    0,
                    usize::MAX,
                    None,
                    true,
                )
            } else {
                let collection_ids = filters
                    .collection
                    .as_deref()
                    .filter(|name| !name.trim().is_empty())
                    .map(|name| {
                        crate::services::collection_service::ids_by_name(
                            &snapshots,
                            &connected,
                            &root,
                            name,
                        )
                    })
                    .transpose()?;
                let (matches, _) = index.indexed_results_filtered_with_category(
                    &filters.query,
                    &filters.search_mode,
                    &filters.tags,
                    collection_ids.as_deref(),
                    filters.favorite_only,
                    filters.model_category.as_deref(),
                )?;
                crate::services::file_service::fetch_matching_files_with_trash(
                    snapshots,
                    connected,
                    &root,
                    filters.media_type,
                    0,
                    usize::MAX,
                    Some(&matches),
                    false,
                )
            };
            let selected_ids = selected_ids.map(|ids| ids.into_iter().collect::<HashSet<_>>());
            Ok(page
                .files
                .into_iter()
                .filter(|file| {
                    file.modified_at_ms
                        .is_some_and(|modified| modified <= snapshot_at_ms)
                })
                .filter(|file| {
                    selected_ids
                        .as_ref()
                        .is_none_or(|ids| ids.contains(&file.id))
                })
                .map(|file| BulkFileTarget {
                    id: file.id,
                    drive_id: file.drive_id,
                    path: file.path,
                })
                .collect::<Vec<_>>())
        })
        .await
        .map_err(AppError::internal)?;
        match &result {
            Ok(targets) => log_event(
                BULK_OPERATION_QUEUE,
                "MATCH-COMPLETE",
                format!("snapshot_at_ms={snapshot_at_ms} matched={}", targets.len()),
            ),
            Err(error) => log_event(
                BULK_OPERATION_QUEUE,
                "MATCH-FAILED",
                format!("snapshot_at_ms={snapshot_at_ms} error={error}"),
            ),
        }
        result
    }
}

fn current_time_ms() -> AppResult<i64> {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(AppError::system_time)?;
    i64::try_from(duration.as_millis()).map_err(AppError::internal)
}
