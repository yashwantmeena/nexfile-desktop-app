use std::error::Error;

use nexfile_desktop_app_lib::AppError;

#[test]
fn serializes_actionable_errors_for_the_ui() {
    let error = AppError::validation("storage quota must be greater than zero");
    let value = serde_json::to_value(error).expect("error should serialize");

    assert_eq!(value["code"], "VALIDATION_ERROR");
    assert_eq!(value["message"], "storage quota must be greater than zero");
}

#[test]
fn hides_internal_details_but_preserves_the_error_source() {
    let error = AppError::from(std::io::Error::other(
        "sensitive path and operating system detail",
    ));
    assert_eq!(
        error
            .source()
            .expect("source should be retained")
            .to_string(),
        "sensitive path and operating system detail"
    );

    let serialized = serde_json::to_string(&error).expect("error should serialize");
    assert!(serialized.contains("FILESYSTEM_ERROR"));
    assert!(!serialized.contains("sensitive path"));
}
