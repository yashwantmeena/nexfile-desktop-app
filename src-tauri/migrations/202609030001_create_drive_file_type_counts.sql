CREATE TABLE IF NOT EXISTS drive_file_type_counts (
    drive_id TEXT NOT NULL,
    file_type TEXT NOT NULL,
    count INTEGER NOT NULL DEFAULT 0 CHECK (typeof(count) = 'integer' AND count >= 0),
    created_at_ms INTEGER NOT NULL DEFAULT (
        CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
    ) CHECK (created_at_ms >= 0),
    updated_at_ms INTEGER NOT NULL DEFAULT (
        CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
    ) CHECK (updated_at_ms >= 0),
    PRIMARY KEY (drive_id, file_type),
    FOREIGN KEY (drive_id) REFERENCES drives(drive_id) ON DELETE CASCADE
);
