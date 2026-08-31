use super::ImageDecoderError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ImagePreparationError {
    #[error(transparent)]
    Decode(#[from] ImageDecoderError),
    #[error("failed to prepare image: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to encode prepared image: {0}")]
    Encode(#[from] image::ImageError),
}

#[derive(Debug, thiserror::Error)]
#[error("the image classifier lock was poisoned")]
pub(crate) struct ClassifierLockPoisoned;

#[derive(Debug, thiserror::Error)]
#[error("the Florence-2 captioner lock was poisoned")]
pub(crate) struct CaptionerLockPoisoned;
