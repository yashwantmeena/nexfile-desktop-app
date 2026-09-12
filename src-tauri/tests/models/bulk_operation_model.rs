use super::BulkOperation;

#[test]
fn serializes_add_to_collection_for_the_frontend_contract() {
    let value = serde_json::to_value(BulkOperation::AddToCollection {
        collection_name: "Travel".to_owned(),
    })
    .unwrap();

    assert_eq!(
        value,
        serde_json::json!({ "addToCollection": { "collectionName": "Travel" } })
    );
}

#[test]
fn serializes_add_tag_for_the_frontend_contract() {
    let value = serde_json::to_value(BulkOperation::AddTag {
        tag: "sunset".to_owned(),
    })
    .unwrap();

    assert_eq!(value, serde_json::json!({ "addTag": { "tag": "sunset" } }));
}
