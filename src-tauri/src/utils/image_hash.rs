use std::f64::consts::PI;
use std::path::Path;

use image::{imageops::FilterType, DynamicImage};

use crate::error::ImageDecoderError;
use crate::utils::image_decoder::decode_image;

const SAMPLE_SIZE: usize = 32;
const HASH_SIZE: usize = 8;

/// Calculates a 64-bit perceptual hash from an already decoded image.
/// Similar-looking images normally have hashes with a small Hamming distance.
pub fn calculate_phash(image: &DynamicImage) -> u64 {
    let grayscale = image
        .resize_exact(SAMPLE_SIZE as u32, SAMPLE_SIZE as u32, FilterType::Lanczos3)
        .to_luma8();
    let mut coefficients = [0.0; HASH_SIZE * HASH_SIZE];

    for frequency_y in 0..HASH_SIZE {
        for frequency_x in 0..HASH_SIZE {
            let mut coefficient = 0.0;
            for y in 0..SAMPLE_SIZE {
                let cosine_y =
                    (PI * (2 * y + 1) as f64 * frequency_y as f64 / (2 * SAMPLE_SIZE) as f64).cos();
                for x in 0..SAMPLE_SIZE {
                    let cosine_x = (PI * (2 * x + 1) as f64 * frequency_x as f64
                        / (2 * SAMPLE_SIZE) as f64)
                        .cos();
                    coefficient +=
                        f64::from(grayscale.get_pixel(x as u32, y as u32)[0]) * cosine_x * cosine_y;
                }
            }
            let scale_x = if frequency_x == 0 {
                std::f64::consts::FRAC_1_SQRT_2
            } else {
                1.0
            };
            let scale_y = if frequency_y == 0 {
                std::f64::consts::FRAC_1_SQRT_2
            } else {
                1.0
            };
            coefficient *= scale_x * scale_y;
            coefficients[frequency_y * HASH_SIZE + frequency_x] = coefficient;
        }
    }

    // The DC coefficient represents average brightness, so it is excluded
    // when selecting the threshold for the structural frequency coefficients.
    let mut frequencies = coefficients[1..].to_vec();
    frequencies.sort_unstable_by(f64::total_cmp);
    let median = frequencies[frequencies.len() / 2];

    coefficients
        .iter()
        .enumerate()
        .fold(0_u64, |hash, (index, coefficient)| {
            hash | (u64::from(*coefficient > median) << index)
        })
}

/// Decodes an image using NexFile's supported-format decoder and calculates
/// its perceptual hash.
pub fn calculate_phash_path(path: &Path) -> Result<u64, ImageDecoderError> {
    decode_image(path).map(|image| calculate_phash(&image))
}

/// Counts the differing bits between two perceptual hashes.
pub const fn phash_distance(left: u64, right: u64) -> u32 {
    (left ^ right).count_ones()
}

/// Formats a perceptual hash as a fixed-width value suitable for JSON or SQL.
pub fn format_phash(hash: u64) -> String {
    format!("{hash:016x}")
}
