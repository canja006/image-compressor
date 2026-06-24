//! AVIF output via the pure-Rust ravif encoder.

use engine::encode::{encode, EncodeFormat};
use image::{DynamicImage, Rgb, RgbImage};

fn detailed(w: u32, h: u32) -> DynamicImage {
    let mut img = RgbImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = Rgb([((x ^ y) & 0xff) as u8, (y & 0xff) as u8, (x & 0xff) as u8]);
    }
    DynamicImage::ImageRgb8(img)
}

#[test]
fn avif_encodes_and_quality_affects_size() {
    let img = detailed(128, 128);
    let lo = encode(&img, EncodeFormat::Avif, Some(15)).unwrap();
    let hi = encode(&img, EncodeFormat::Avif, Some(90)).unwrap();
    assert!(
        !lo.is_empty() && !hi.is_empty(),
        "AVIF output should be produced"
    );
    assert!(
        hi.len() > lo.len(),
        "higher quality should be larger: {} vs {}",
        hi.len(),
        lo.len()
    );
    // An AVIF file is an ISO-BMFF container: bytes 4..8 are the 'ftyp' box type.
    assert_eq!(&hi[4..8], b"ftyp");
}
