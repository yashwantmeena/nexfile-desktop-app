use crate::repositories::collection_repository::*;
#[tokio::test]
async fn collection_timestamps_preserve_creation_and_no_op_updates() {
    let root = std::env::temp_dir().join(format!("collection-times-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db = crate::repositories::database_repository::SqliteDatabase::open(root.join("test.db"))
        .await
        .unwrap();
    let created = save(db.pool(), None, "Travel").await.unwrap().remove(0);
    assert!(created.created_at_ms > 0);
    assert_eq!(created.created_at_ms, created.updated_at_ms);
    sqlx::query("UPDATE collections SET updated_at_ms = 1 WHERE id = ?")
        .bind(&created.id)
        .execute(db.pool())
        .await
        .unwrap();
    let unchanged = save(db.pool(), Some(&created.id), "Travel")
        .await
        .unwrap()
        .remove(0);
    assert_eq!(unchanged.updated_at_ms, 1);
    let renamed = save(db.pool(), Some(&created.id), "Trips")
        .await
        .unwrap()
        .remove(0);
    assert_eq!(renamed.created_at_ms, created.created_at_ms);
    assert!(renamed.updated_at_ms > 1);
    sqlx::query("INSERT INTO collections (id, name, name_key) VALUES ('legacy', 'Old', 'old')")
        .execute(db.pool())
        .await
        .unwrap();
    let legacy = list(db.pool())
        .await
        .unwrap()
        .into_iter()
        .find(|item| item.id == "legacy")
        .unwrap();
    assert_eq!(legacy.created_at_ms, 0);
    db.close().await;
    std::fs::remove_dir_all(root).unwrap();
}
