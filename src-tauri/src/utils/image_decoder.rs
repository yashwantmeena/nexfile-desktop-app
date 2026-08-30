use std::{
    path::Path,
    sync::{Arc, OnceLock},
};

use image::DynamicImage;

use crate::error::ImageDecoderError;

const MIN_SVG_RENDER_DIMENSION: f32 = 512.0;
const MAX_SVG_RENDER_DIMENSION: f32 = 2048.0;
static SVG_FONT_DATABASE: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();

pub fn decode_image(path: &Path) -> Result<DynamicImage, ImageDecoderError> {
    match extension(path).as_deref() {
        Some("avif" | "avip") => decode_avif(path),
        Some("heic" | "heif") => decode_heif(path),
        Some("svg") => decode_svg(path),
        _ => Ok(image::open(path)?),
    }
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
}

fn decode_heif(path: &Path) -> Result<DynamicImage, ImageDecoderError> {
    let decoded = heif_oxide::decode_file(path)
        .map_err(|error| ImageDecoderError::Heif(error.to_string()))?;
    let (width, height) = (decoded.width, decoded.height);
    let pixels = decoded.to_rgba8();
    let image = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| ImageDecoderError::Heif("decoder returned an invalid RGBA buffer".into()))?;
    Ok(DynamicImage::ImageRgba8(image))
}

fn decode_svg(path: &Path) -> Result<DynamicImage, ImageDecoderError> {
    let data = std::fs::read(path)?;
    let mut options = resvg::usvg::Options::default();
    options.resources_dir = path.parent().map(Path::to_path_buf);
    options.fontdb = svg_font_database();
    let tree = resvg::usvg::Tree::from_data(&data, &options)
        .map_err(|error| ImageDecoderError::Svg(error.to_string()))?;
    let size = tree.size();
    let largest_dimension = size.width().max(size.height());
    if !largest_dimension.is_finite() || largest_dimension <= 0.0 {
        return Err(ImageDecoderError::Svg(
            "SVG has invalid or zero dimensions".into(),
        ));
    }

    let rendered_dimension =
        largest_dimension.clamp(MIN_SVG_RENDER_DIMENSION, MAX_SVG_RENDER_DIMENSION);
    let scale = rendered_dimension / largest_dimension;
    let width = (size.width() * scale).ceil().max(1.0) as u32;
    let height = (size.height() * scale).ceil().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| ImageDecoderError::Svg("SVG render dimensions are too large".into()))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    let pixels = pixmap.take_demultiplied();
    let image = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| ImageDecoderError::Svg("renderer returned an invalid RGBA buffer".into()))?;
    Ok(DynamicImage::ImageRgba8(image))
}

fn svg_font_database() -> Arc<resvg::usvg::fontdb::Database> {
    SVG_FONT_DATABASE
        .get_or_init(|| {
            let mut database = resvg::usvg::fontdb::Database::new();
            database.load_system_fonts();
            Arc::new(database)
        })
        .clone()
}

fn decode_avif(path: &Path) -> Result<DynamicImage, ImageDecoderError> {
    let decoded = avif_decode::Decoder::from_avif(&std::fs::read(path)?)
        .and_then(avif_decode::Decoder::to_image)
        .map_err(|error| ImageDecoderError::Avif(error.to_string()))?;

    let image = match decoded {
        avif_decode::Image::Rgb8(image) => {
            let (width, height) = (image.width() as u32, image.height() as u32);
            let pixels = image
                .pixels()
                .flat_map(|pixel| [pixel.r, pixel.g, pixel.b])
                .collect();
            DynamicImage::ImageRgb8(image::RgbImage::from_raw(width, height, pixels).ok_or_else(
                || ImageDecoderError::Avif("decoder returned an invalid RGB8 buffer".into()),
            )?)
        }
        avif_decode::Image::Rgba8(image) => {
            let (width, height) = (image.width() as u32, image.height() as u32);
            let pixels = image
                .pixels()
                .flat_map(|pixel| [pixel.r, pixel.g, pixel.b, pixel.a])
                .collect();
            DynamicImage::ImageRgba8(
                image::RgbaImage::from_raw(width, height, pixels).ok_or_else(|| {
                    ImageDecoderError::Avif("decoder returned an invalid RGBA8 buffer".into())
                })?,
            )
        }
        avif_decode::Image::Rgb16(image) => {
            let (width, height) = (image.width() as u32, image.height() as u32);
            let pixels = image
                .pixels()
                .flat_map(|pixel| [pixel.r, pixel.g, pixel.b])
                .collect();
            DynamicImage::ImageRgb16(
                image::ImageBuffer::from_raw(width, height, pixels).ok_or_else(|| {
                    ImageDecoderError::Avif("decoder returned an invalid RGB16 buffer".into())
                })?,
            )
        }
        avif_decode::Image::Rgba16(image) => {
            let (width, height) = (image.width() as u32, image.height() as u32);
            let pixels = image
                .pixels()
                .flat_map(|pixel| [pixel.r, pixel.g, pixel.b, pixel.a])
                .collect();
            DynamicImage::ImageRgba16(
                image::ImageBuffer::from_raw(width, height, pixels).ok_or_else(|| {
                    ImageDecoderError::Avif("decoder returned an invalid RGBA16 buffer".into())
                })?,
            )
        }
        avif_decode::Image::Gray8(image) => {
            let (width, height) = (image.width() as u32, image.height() as u32);
            let pixels = image.pixels().map(|pixel| pixel.value()).collect();
            DynamicImage::ImageLuma8(
                image::GrayImage::from_raw(width, height, pixels).ok_or_else(|| {
                    ImageDecoderError::Avif("decoder returned an invalid grayscale buffer".into())
                })?,
            )
        }
        avif_decode::Image::Gray16(image) => {
            let (width, height) = (image.width() as u32, image.height() as u32);
            let pixels = image.pixels().map(|pixel| pixel.value()).collect();
            DynamicImage::ImageLuma16(
                image::ImageBuffer::from_raw(width, height, pixels).ok_or_else(|| {
                    ImageDecoderError::Avif("decoder returned an invalid grayscale buffer".into())
                })?,
            )
        }
    };

    Ok(image)
}

#[cfg(test)]
#[path = "../../tests/utils/image_decoder.rs"]
mod tests;
