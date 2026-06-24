//! Encoder behavior: quality affects JPEG size, PNG is lossless, alpha is flattened for JPEG.

use engine::decode::decode;
use engine::encode::{encode, EncodeFormat};
use image::{DynamicImage, Rgb, RgbImage, Rgba, RgbaImage};

fn jpeg() -> EncodeFormat {
    EncodeFormat::Jpeg {
        background: [255, 255, 255],
    }
}

fn detailed(w: u32, h: u32) -> DynamicImage {
    let mut img = RgbImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = Rgb([((x ^ y) & 0xff) as u8, (y & 0xff) as u8, (x & 0xff) as u8]);
    }
    DynamicImage::ImageRgb8(img)
}

#[test]
fn jpeg_size_grows_with_quality() {
    let img = detailed(256, 256);
    let lo = encode(&img, jpeg(), Some(10)).unwrap().len();
    let hi = encode(&img, jpeg(), Some(90)).unwrap().len();
    assert!(hi > lo, "higher quality should be larger: {hi} vs {lo}");
}

#[test]
fn extensions_are_correct() {
    assert_eq!(jpeg().extension(), "jpg");
    assert_eq!(EncodeFormat::Png.extension(), "png");
}

#[test]
fn png_is_lossless_roundtrip() {
    let img = detailed(64, 64);
    let bytes = encode(&img, EncodeFormat::Png, None).unwrap();
    let decoded = decode(&bytes).unwrap();
    assert_eq!(decoded.to_rgb8(), img.to_rgb8());
}

#[test]
fn alpha_is_flattened_onto_background_for_jpeg() {
    // A fully transparent source must render as the background color in JPEG (which has no alpha).
    let mut img = RgbaImage::new(16, 16);
    for p in img.pixels_mut() {
        *p = Rgba([10, 20, 30, 0]);
    }
    let bg = [200, 100, 50];
    let bytes = encode(
        &DynamicImage::ImageRgba8(img),
        EncodeFormat::Jpeg { background: bg },
        Some(95),
    )
    .unwrap();
    let decoded = decode(&bytes).unwrap().to_rgb8();
    let px = decoded.get_pixel(8, 8).0;
    // JPEG is lossy, so allow a small tolerance around the background color.
    for (got, want) in px.iter().zip(bg.iter()) {
        assert!(
            (i32::from(*got) - i32::from(*want)).abs() <= 8,
            "got {got}, want {want}"
        );
    }
}
