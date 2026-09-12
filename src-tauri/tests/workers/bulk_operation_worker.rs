use std::path::PathBuf;

use super::*;
use crate::models::bulk_operation_model::BulkFileTarget;

fn target() -> BulkFileTarget {
    BulkFileTarget {
        drive_id: "drive-1".to_owned(),
        file_id: "file-1".to_owned(),
        path: PathBuf::from("file-1.jpg"),
    }
}

#[test]
fn delete_dispatches_a_move_to_trash_metadata_job() {
    let job = file_processing_job(&BulkOperation::Delete, "process-1", target());

    assert_eq!(
        job,
        FileProcessingJob::UpdateMetadata(UpdateFileMetadataJob {
            process_id: "process-1".to_owned(),
            drive_id: "drive-1".to_owned(),
            path: PathBuf::from("file-1.jpg"),
            favorite: None,
            is_trashed: Some(true),
            add_tags: Vec::new(),
            add_collection_names: Vec::new(),
        })
    );
}

#[test]
fn empty_trash_dispatches_a_permanent_delete_job() {
    let job = file_processing_job(&BulkOperation::EmptyTrash, "process-1", target());

    assert_eq!(
        job,
        FileProcessingJob::Delete(DeleteFileJob {
            process_id: "process-1".to_owned(),
            drive_id: "drive-1".to_owned(),
            file_id: "file-1".to_owned(),
            path: PathBuf::from("file-1.jpg"),
        })
    );
}

#[test]
fn add_to_collection_dispatches_an_append_membership_job() {
    let job = file_processing_job(
        &BulkOperation::AddToCollection {
            collection_name: "Travel".to_owned(),
        },
        "process-1",
        target(),
    );

    assert_eq!(
        job,
        FileProcessingJob::UpdateMetadata(UpdateFileMetadataJob {
            process_id: "process-1".to_owned(),
            drive_id: "drive-1".to_owned(),
            path: PathBuf::from("file-1.jpg"),
            favorite: None,
            is_trashed: None,
            add_tags: Vec::new(),
            add_collection_names: vec!["Travel".to_owned()],
        })
    );
}

#[test]
fn add_tag_dispatches_through_the_metadata_update_job() {
    let job = file_processing_job(
        &BulkOperation::AddTag {
            tag: "sunset".to_owned(),
        },
        "process-1",
        target(),
    );

    assert_eq!(
        job,
        FileProcessingJob::UpdateMetadata(UpdateFileMetadataJob {
            process_id: "process-1".to_owned(),
            drive_id: "drive-1".to_owned(),
            path: PathBuf::from("file-1.jpg"),
            favorite: None,
            is_trashed: None,
            add_tags: vec!["sunset".to_owned()],
            add_collection_names: Vec::new(),
        })
    );
}
