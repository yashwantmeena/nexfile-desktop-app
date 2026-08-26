CREATE TABLE IF NOT EXISTS drives (
    drive_id TEXT PRIMARY KEY NOT NULL,
    drive_name TEXT NOT NULL,
    partition_name TEXT NOT NULL,
    app_limit_bytes INTEGER CHECK (app_limit_bytes >= 0),
    file_count INTEGER NOT NULL CHECK (file_count >= 0),
    app_used_bytes INTEGER NOT NULL CHECK (app_used_bytes >= 0),
    priority INTEGER NOT NULL DEFAULT 0 CHECK (priority >= 0),
    is_mounted INTEGER NOT NULL DEFAULT 0 CHECK (is_mounted IN (0, 1)),
    created_at_ms INTEGER NOT NULL DEFAULT (
        CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
    ) CHECK (created_at_ms >= 0),
    updated_at_ms INTEGER NOT NULL DEFAULT (
        CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
    ) CHECK (updated_at_ms >= 0)
);
