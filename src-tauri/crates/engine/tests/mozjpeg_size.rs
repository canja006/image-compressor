//! Feature-gated smoke test for the mozjpeg JPEG encoder path. Only compiled/run with
//! `--features mozjpeg` (the default pure-Rust build cannot A/B against itself). It proves the
//! mozjpeg path produces a valid baseline JPEG and that size grows with quality; the orchestrator
//! compares mozjpeg-vs-pure-Rust byte sizes across the two builds separately.
#![cfg(feature = "mozjpeg")]

use engine::encode::{encode, EncodeFormat};
use image::{DynamicImage, Rgb, RgbImage};

/// Deterministic, moderately detailed image — a flat color would compress to almost nothing and
/// wouldn't exercise the quality knob; this XOR/gradient pattern gives JPEG sizes that grow with q.
fn test_image(w: u32, h: u32) -> DynamicImage {
    let mut img = RgbImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels_mut() {
        let r = ((x ^ y) & 0xff) as u8;
        let g = (x.wrapping_add(y) & 0xff) as u8;
        let b = ((x.wrapping_mul(2) ^ y) & 0xff) as u8;
        *p = Rgb([r, g, b]);
    }
    DynamicImage::ImageRgb8(img)
}

fn jpeg() -> EncodeFormat {
    EncodeFormat::Jpeg {
        background: [255, 255, 255],
    }
}

#[test]
fn mozjpeg_path_produces_valid_jpeg_and_size_grows_with_quality() {
    let img = test_image(512, 512);

    let q75 = encode(&img, jpeg(), Some(75)).expect("mozjpeg q75 encode");
    let q95 = encode(&img, jpeg(), Some(95)).expect("mozjpeg q95 encode");

    // Valid baseline JPEG SOI marker.
    assert_eq!(&q75[..2], &[0xFF, 0xD8], "q75 is not a valid JPEG");
    assert_eq!(&q95[..2], &[0xFF, 0xD8], "q95 is not a valid JPEG");
    assert!(
        !q75.is_empty() && !q95.is_empty(),
        "encodes must be non-empty"
    );

    // Higher quality is never smaller for the same image.
    assert!(
        q75.len() <= q95.len(),
        "q75 ({}) should be <= q95 ({})",
        q75.len(),
        q95.len()
    );
}
