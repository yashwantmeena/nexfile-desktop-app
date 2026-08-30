use super::*;

fn test_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("nexfile-image-decoder-{}", uuid::Uuid::new_v4()))
}

#[test]
fn decodes_ico_images() {
    let root = test_root();
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let path = root.join("icon.ico");
    DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        32,
        32,
        image::Rgba([255, 0, 0, 255]),
    ))
    .save_with_format(&path, image::ImageFormat::Ico)
    .expect("ICO fixture should encode");

    let decoded = decode_image(&path).expect("ICO should decode");
    assert_eq!((decoded.width(), decoded.height()), (32, 32));

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[test]
fn rasterizes_svg_images() {
    let root = test_root();
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let path = root.join("vector.svg");
    std::fs::write(
        &path,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="40">
             <rect width="80" height="40" fill="#ff0000"/>
           </svg>"##,
    )
    .expect("SVG fixture should be written");

    let decoded = decode_image(&path).expect("SVG should rasterize");
    assert_eq!((decoded.width(), decoded.height()), (512, 256));

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[test]
fn decodes_heic_and_heif_images() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("flat_red_64.heic");
    let heic = decode_image(&fixture).expect("HEIC should decode");
    assert_eq!((heic.width(), heic.height()), (64, 64));

    let root = test_root();
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let heif_path = root.join("flat-red.heif");
    std::fs::copy(&fixture, &heif_path).expect("HEIF alias fixture should be copied");
    let heif = decode_image(&heif_path).expect("HEIF should decode");
    assert_eq!((heif.width(), heif.height()), (64, 64));

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}
