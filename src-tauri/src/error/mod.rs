pub mod app_error;
pub mod clip_error;
pub mod florence2_error;
pub mod image_decoder_error;
pub(crate) mod image_processing_error;
pub(crate) mod import_error;
pub mod result;

pub use app_error::AppError;
pub use clip_error::ClipError;
pub use florence2_error::Florence2Error;
pub use image_decoder_error::ImageDecoderError;
pub(crate) use image_processing_error::{
    CaptionerLockPoisoned, ClassifierLockPoisoned, ImagePreparationError,
};
pub(crate) use import_error::CounterOverflow;
pub use result::AppResult;
