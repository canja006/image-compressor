//! Extra target-search cases: a generous cap keeps max quality at full size; a lossless PNG
//! still meets a tight cap purely by downscaling.

use engine::encode::{encode, EncodeFormat};
use engine::{compress_to_target, Options};
use image::{DynamicImage, Rgb, RgbImage};

fn detailed(w: u32, h: u32) -> DynamicImage {
    let mut img = RgbImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = Rgb([((x ^ y) & 0xff) as u8, (y & 0xff) as u8, (x & 0xff) as u8]);
    }
    DynamicImage::ImageRgb8(img)
}

fn jpeg() -> EncodeFormat {
    EncodeFormat::Jpeg {
        background: [255, 255, 255],
    }
}

#[test]
fn generous_cap_keeps_max_quality_and_full_size() {
    let img = detailed(128, 128);
    let opts = Options::default();
    let cap = encode(&img, jpeg(), Some(opts.jpeg_quality_max))
        .unwrap()
        .len() as u64
        + 10_000;

    let res = compress_to_target(&img, cap, jpeg(), &opts, true)
        .unwrap()
        .expect("a generous cap is reachable");
    assert!(!res.downscaled, "no downscaling needed");
    assert_eq!(res.quality, Some(opts.jpeg_quality_max));
}

#[test]
fn lossless_png_meets_a_tight_cap_via_downscale() {
    let img = detailed(800, 800);
    let opts = Options::default();
    let full = encode(&img, EncodeFormat::Png, None).unwrap().len() as u64;
    let cap = full / 4;

    let res = compress_to_target(&img, cap, EncodeFormat::Png, &opts, true)
        .unwrap()
        .expect("reachable by downscaling");
    assert!(res.downscaled, "PNG has no quality knob, so it must shrink");
    assert!(res.quality.is_none());
    assert!(res.bytes.len() as u64 <= cap, "output exceeds the cap");
}
