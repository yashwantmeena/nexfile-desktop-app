use std::path::Path;

use crate::utils::image_decoder::is_supported_image;

#[test]
fn recognizes_every_classifiable_image_extension() {
    for extension in [
        "avif", "avip", "bmp", "gif", "heic", "heif", "ico", "jpeg", "jpg", "png", "svg", "tif",
        "tiff", "webp",
    ] {
        assert!(
            is_supported_image(Path::new(&format!("image.{extension}"))),
            "expected .{extension} to be supported"
        );
    }
    assert!(!is_supported_image(Path::new("document.pdf")));
}
