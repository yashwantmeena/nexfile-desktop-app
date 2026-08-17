use nexfile_desktop_app_lib::{Florence2Config, Florence2ModelPaths, Florence2Task};

#[test]
fn resolves_standard_and_quantized_model_paths() {
    let paths = Florence2ModelPaths::from_dir("florence-2-base-ft");
    assert_eq!(
        paths.vision_encoder,
        std::path::PathBuf::from("florence-2-base-ft/onnx/vision_encoder.onnx")
    );
    assert_eq!(
        paths.tokenizer,
        std::path::PathBuf::from("florence-2-base-ft/tokenizer.json")
    );

    let quantized = Florence2ModelPaths::from_dir_with_suffix("florence-2-base-ft", Some("_int8"));
    assert_eq!(
        quantized.decoder,
        std::path::PathBuf::from("florence-2-base-ft/onnx/decoder_model_int8.onnx")
    );
}

#[test]
fn constructs_official_task_prompts() {
    assert_eq!(
        Florence2Task::Caption.prompt(),
        "What does the image describe?"
    );
    assert_eq!(
        Florence2Task::OpenVocabularyDetection("red car".into()).prompt(),
        "Locate red car in the image."
    );
}

#[test]
fn uses_florence2_preprocessing_and_generation_defaults() {
    let config = Florence2Config::default();
    assert_eq!(config.image_size, 768);
    assert_eq!(config.max_prompt_tokens, 447);
    assert_eq!(config.decoder_start_token_id, 2);
    assert_eq!(config.eos_token_id, 2);
}
