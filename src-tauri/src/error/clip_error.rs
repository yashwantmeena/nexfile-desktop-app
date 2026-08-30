use std::path::PathBuf;

use super::ImageDecoderError;

#[derive(Debug, thiserror::Error)]
pub enum ClipError {
    #[error("required CLIP file does not exist: {0}")]
    MissingFile(PathBuf),
    #[error("invalid CLIP configuration: {0}")]
    InvalidConfig(String),
    #[error("incompatible CLIP ONNX export: {0}")]
    IncompatibleModel(String),
    #[error("failed to load or preprocess image: {0}")]
    Image(#[from] ImageDecoderError),
    #[error("ONNX Runtime error: {0}")]
    Onnx(#[from] ort::Error),
    #[error("CLIP tokenizer error: {0}")]
    Tokenizer(String),
    #[error("cannot compare embeddings with dimensions {left} and {right}")]
    DimensionMismatch { left: usize, right: usize },
    #[error("the model returned an empty or zero-length embedding")]
    EmptyEmbedding,
}
