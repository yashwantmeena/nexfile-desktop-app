use crate::utils::image_hash::*;
use image::imageops::FilterType;
use image::DynamicImage;

fn patterned_image(width: u32, height: u32, inverted: bool) -> DynamicImage {
    DynamicImage::ImageLuma8(image::GrayImage::from_fn(width, height, |x, y| {
        let value = ((x * 13 + y * 7 + (x * y) % 251) % 256) as u8;
        image::Luma([if inverted { 255 - value } else { value }])
    }))
}

#[test]
fn produces_a_stable_fixed_width_phash() {
    let image = patterned_image(96, 64, false);
    let first = calculate_phash(&image);
    let second = calculate_phash(&image);

    assert_eq!(first, second);
    assert_eq!(format_phash(first).len(), 16);
}

#[test]
fn is_independent_of_image_dimensions() {
    let original = patterned_image(96, 64, false);
    let resized = original.resize_exact(192, 128, FilterType::Lanczos3);

    assert!(phash_distance(calculate_phash(&original), calculate_phash(&resized)) <= 5);
}

#[test]
fn distinguishes_visually_different_patterns() {
    let first = calculate_phash(&patterned_image(96, 64, false));
    let second = calculate_phash(&patterned_image(96, 64, true));

    assert!(phash_distance(first, second) > 5);
    assert_eq!(phash_distance(first, second), phash_distance(second, first));
    assert_eq!(phash_distance(first, first), 0);
}

#[test]
fn calculates_a_hash_from_a_supported_image_path() {
    let root = std::env::temp_dir().join(format!("nexfile-phash-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let path = root.join("image.png");
    let image = patterned_image(96, 64, false);
    image.save(&path).expect("PNG fixture should be written");

    assert_eq!(
        calculate_phash_path(&path).expect("image hash should be calculated"),
        calculate_phash(&image)
    );

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}
