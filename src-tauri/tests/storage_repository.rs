use std::sync::atomic::{AtomicU64, Ordering};

use nexfile_desktop_app_lib::{RedbStorageRepository, StorageAllocation};
use redb::{Database, TableDefinition};

static NEXT_TEST_DATABASE: AtomicU64 = AtomicU64::new(0);
const STORAGE_ALLOCATIONS_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("storage_allocations");

fn test_database_path() -> std::path::PathBuf {
    let unique = NEXT_TEST_DATABASE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "nexfile-storage-{}-{unique}.redb",
        std::process::id()
    ))
}

#[test]
fn persists_and_clears_storage_allocations() {
    let path = test_database_path();
    let allocation = StorageAllocation {
        schema_version: 1,
        priority: 1,
        volume_id: "volume-a".into(),
        physical_disk_id: Some("PHYSICALDRIVE0".into()),
        hardware_model: Some("Test SSD".into()),
        volume_label: Some("Archive".into()),
        mount_point: "D:\\".into(),
        quota_bytes: 500,
        vault_used_bytes: 100,
        updated_at_unix_ms: 1_700_000_000_000,
    };

    {
        let repository = RedbStorageRepository::open(&path).expect("test database should open");
        assert!(repository.list().expect("read should succeed").is_empty());

        repository
            .save_all(std::slice::from_ref(&allocation))
            .expect("save should succeed");
        assert_eq!(
            repository.list().expect("read should succeed"),
            vec![allocation]
        );

        repository.clear().expect("clear should succeed");
        assert!(repository.list().expect("read should succeed").is_empty());
    }

    std::fs::remove_file(path).expect("test database should be removable");
}

#[test]
fn clears_allocations_when_persisted_json_is_malformed() {
    let path = test_database_path();

    {
        let database = Database::create(&path).expect("test database should open");
        let write = database.begin_write().expect("write should begin");
        {
            let mut table = write
                .open_table(STORAGE_ALLOCATIONS_TABLE)
                .expect("table should open");
            table
                .insert("broken-volume", b"{not valid json".as_slice())
                .expect("malformed record should be inserted");
        }
        write.commit().expect("write should commit");
    }

    {
        let repository = RedbStorageRepository::open(&path).expect("repository should open");
        assert!(repository.list().is_err());

        repository
            .clear()
            .expect("malformed records should still be clearable");
        assert!(repository.list().expect("read should succeed").is_empty());
    }

    std::fs::remove_file(path).expect("test database should be removable");
}
