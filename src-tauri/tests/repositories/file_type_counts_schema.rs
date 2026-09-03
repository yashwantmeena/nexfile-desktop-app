use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[tokio::test]
async fn creates_row_based_counts_with_cascade_deletion() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .in_memory(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../migrations/202608260001_create_drives.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../../migrations/202609030001_create_drive_file_type_counts.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(
        "INSERT INTO drives (drive_id, drive_name, partition_name, file_count, app_used_bytes)
        VALUES ('test', 'Test', 'Test', 21, 100);
        INSERT INTO drive_file_type_counts (drive_id, file_type, count) VALUES
            ('test', 'image', 1), ('test', 'video', 2), ('test', 'audio', 3),
            ('test', 'document', 4), ('test', 'archive', 5), ('test', 'other', 6);",
    )
    .execute(&pool)
    .await
    .unwrap();

    let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT file_type, count FROM drive_file_type_counts ORDER BY file_type",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows,
        vec![
            ("archive".into(), 5),
            ("audio".into(), 3),
            ("document".into(), 4),
            ("image".into(), 1),
            ("other".into(), 6),
            ("video".into(), 2)
        ]
    );
    let timestamps = sqlx::query_as::<_, (i64, i64)>(
        "SELECT created_at_ms, updated_at_ms FROM drive_file_type_counts",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(timestamps.len(), 6);
    assert!(timestamps
        .iter()
        .all(|(created, updated)| *created > 0 && created == updated));
    // The table accepts a future category without adding a new column.
    sqlx::query("INSERT INTO drive_file_type_counts (drive_id, file_type, count) VALUES ('test', 'future-category', 7)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM drives WHERE drive_id = 'test'")
        .execute(&pool)
        .await
        .unwrap();
    for query in ["SELECT COUNT(*) FROM drive_file_type_counts"] {
        let count = sqlx::query_scalar::<_, i64>(query)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
    pool.close().await;
}
