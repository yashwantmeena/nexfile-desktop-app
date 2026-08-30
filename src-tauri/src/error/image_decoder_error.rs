#[derive(Debug, thiserror::Error)]
pub enum ImageDecoderError {
    #[error("failed to read image: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to decode raster image: {0}")]
    Raster(#[from] image::ImageError),
    #[error("failed to decode AVIF image: {0}")]
    Avif(String),
    #[error("failed to decode HEIC/HEIF image: {0}")]
    Heif(String),
    #[error("failed to render SVG image: {0}")]
    Svg(String),
}
