CREATE TABLE IF NOT EXISTS background_processes (
    process_id TEXT PRIMARY KEY NOT NULL,
    process_type TEXT NOT NULL,
    status TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0 CHECK (priority >= 0),
    total_items INTEGER NOT NULL DEFAULT 0 CHECK (total_items >= 0),
    processed_items INTEGER NOT NULL DEFAULT 0 CHECK (
        processed_items >= 0 AND processed_items <= total_items
    ),
    failed_items INTEGER NOT NULL DEFAULT 0 CHECK (
        failed_items >= 0 AND failed_items <= processed_items
    ),
    remark TEXT,
    created_at_ms INTEGER NOT NULL DEFAULT (
        CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
    ) CHECK (created_at_ms >= 0),
    updated_at_ms INTEGER NOT NULL DEFAULT (
        CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
    ) CHECK (updated_at_ms >= 0),
    started_at_ms INTEGER CHECK (started_at_ms >= 0),
    finished_at_ms INTEGER CHECK (finished_at_ms >= 0)
);
