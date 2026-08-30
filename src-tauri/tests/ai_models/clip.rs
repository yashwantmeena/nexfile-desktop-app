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

#[test]
#[ignore = "loads the bundled CLIP model and runs ONNX inference"]
fn bundled_model_runs_image_and_text_inference() {
    let model_directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("ai-models")
        .join("clip-vit-base-patch32");
    let mut model = ClipModel::load(ClipModelPaths::from_dir(model_directory))
        .expect("bundled CLIP model should load");

    let text = model
        .embed_text("A solid red image.")
        .expect("text inference should succeed");
    let image = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        224,
        224,
        image::Rgb([255, 0, 0]),
    ));
    let image = model
        .embed_image(&image)
        .expect("image inference should succeed");

    assert!(!text.is_empty());
    assert_eq!(text.len(), image.len());
    assert!(ClipModel::cosine_similarity(&text, &image)
        .expect("embeddings should be comparable")
        .is_finite());
}
