# Collection storage and import

The app catalog is stored in SQLite `collections` (14-character Nano ID, name, normalized unique name). Creating, renaming, and deleting catalog entries does not write drive metadata. No drive needs to be connected to create a collection.

Import offers optional collection checkboxes for both files and folders. Selected catalog IDs are validated before queueing. Names are captured as a JSON array in `background_processes.collections`, so queued jobs and retries retain their choices across restarts. Later catalog renames/deletions do not change already queued imports.

Only after a file is published to its actual destination does the importer add its collection memberships to `nexfile/collections.json`. It reuses a matching name on that drive or creates a new drive-local 14-character Nano ID. The app ID is not copied onto the drive. Capacity fallback therefore writes collections only on the drive that received the file. Imports without selected or inherited collections do not create the metadata file.

The drive document contains `version`, `driveId`, and `collections` (local ID, name, timestamps). Each managed file sidecar (`files/<id>.<extension>.json`) stores `collectionIds`. Imports read and update only the known file sidecar. There is no drive-wide membership map or scan in the collection service. Drive-wide discovery belongs to indexing; the app SQLite database is not needed to resolve sidecar IDs using drive definitions. AI processing preserves collectionIds. Old fileCollections maps are ignored; no migration or recovery map is written. Imports save definitions only when creating a new drive-local collection, before updating the destination sidecar; import retries finish interrupted writes. Existing managed-file imports also inherit their source collection names. Repeated jobs deduplicate membership. Writes use the import metadata lock and atomic file replacement. File publication and collection metadata are retryable, not one filesystem transaction.

Catalog rename/delete affects future import choices only. Existing drive collections remain independent. Old drive-only collections are preserved on disk; they are not automatically migrated into the app catalog. Membership-aware file browsing remains separate work.

Collection IPC is CRUD only: `create_collection` (name), `list_collections`, `update_collection` (id, name), `delete_collection` (id). `import_file` and `import_folder` accept optional `collectionIds`. Drive-local metadata is maintained internally by the import service, not exposed as separate collection commands.

Collections expose createdAtMs and updatedAtMs as UTC Unix milliseconds in IPC and drive JSON (created_at_ms / updated_at_ms in SQLite). Creation initializes both; a definition rename updates updatedAtMs. Idempotent retries do not change timestamps. Existing rows and old drive entries default missing timestamps to 0 (unknown); their historical creation times are not invented. Catalog and drive-local collections have independent creation times.







