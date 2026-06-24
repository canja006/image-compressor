//! Decoding: valid buffers round-trip; corrupt buffers return an error, never a panic.

use engine::decode::decode;
use engine::encode::{encode, EncodeFormat};
use image::{DynamicImage, Rgb, RgbImage};

#[test]
fn decodes_a_valid_image() {
    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(8, 8, Rgb([1, 2, 3])));
    let bytes = encode(&img, EncodeFormat::Png, None).unwrap();
    let decoded = decode(&bytes).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (8, 8));
}

#[test]
fn corrupt_bytes_return_an_error() {
    assert!(decode(b"this is not an image").is_err());
    assert!(decode(&[]).is_err());
}
