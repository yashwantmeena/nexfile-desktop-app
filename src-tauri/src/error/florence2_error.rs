use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Florence2Error {
    #[error("required Florence-2 file does not exist: {0}")]
    MissingFile(PathBuf),
    #[error("invalid Florence-2 configuration: {0}")]
    InvalidConfig(String),
    #[error("incompatible Florence-2 ONNX export: {0}")]
    IncompatibleModel(String),
    #[error("failed to load or preprocess image: {0}")]
    Image(#[from] image::ImageError),
    #[error("ONNX Runtime error: {0}")]
    Onnx(#[from] ort::Error),
    #[error("Florence-2 tokenizer error: {0}")]
    Tokenizer(String),
    #[error("Florence-2 tensor error: {0}")]
    Tensor(String),
    #[error("Florence-2 returned no usable token logits")]
    EmptyLogits,
}
