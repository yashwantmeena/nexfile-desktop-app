use nexfile_desktop_app_lib::{ClipConfig, ClipError, ClipModel, ClipModelPaths};

#[test]
fn resolves_conventional_model_paths() {
    let paths = ClipModelPaths::from_dir("ai-models/clip-vit-base-patch32");
    assert_eq!(
        paths.vision_model,
        std::path::PathBuf::from("ai-models/clip-vit-base-patch32/vision_model.onnx")
    );
    assert_eq!(
        paths.text_model,
        std::path::PathBuf::from("ai-models/clip-vit-base-patch32/text_model.onnx")
    );
    assert_eq!(
        paths.tokenizer,
        std::path::PathBuf::from("ai-models/clip-vit-base-patch32/tokenizer.json")
    );
}

#[test]
fn uses_clip_preprocessing_defaults() {
    let config = ClipConfig::default();
    assert_eq!(config.image_size, 224);
    assert_eq!(config.context_length, 77);
}

#[test]
fn computes_cosine_similarity() {
    let score = ClipModel::cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).unwrap();
    assert_eq!(score, 0.0);
}

#[test]
fn rejects_mismatched_embedding_dimensions() {
    let error = ClipModel::cosine_similarity(&[1.0], &[1.0, 0.0]).unwrap_err();
    assert!(matches!(error, ClipError::DimensionMismatch { .. }));
}
