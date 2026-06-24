//! In-memory single-image preview used for the live before/after readout.

use engine::encode::{encode, EncodeFormat};
use engine::model::{Options, OutputFormat};
use engine::preview;
use image::{DynamicImage, Rgb, RgbImage};
use std::path::{Path, PathBuf};

fn unique_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ic_preview_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_jpeg(path: &Path, w: u32, h: u32) {
    let mut img = RgbImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = Rgb([((x ^ y) & 0xff) as u8, (y & 0xff) as u8, (x & 0xff) as u8]);
    }
    let bytes = encode(
        &DynamicImage::ImageRgb8(img),
        EncodeFormat::Jpeg {
            background: [255, 255, 255],
        },
        Some(95),
    )
    .unwrap();
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn preview_reports_metrics_and_bytes() {
    let dir = unique_dir("ok");
    let src = dir.join("p.jpg");
    write_jpeg(&src, 600, 400);
    let original = std::fs::metadata(&src).unwrap().len();
    let opts = Options {
        cap_bytes: original / 3,
        output_format: OutputFormat::Jpeg,
        skip_if_under_cap: false,
        ..Options::default()
    };

    let p = preview(&src, &opts);
    assert_eq!(p.kind, "compressed");
    assert_eq!(p.source_width, 600);
    assert_eq!(p.source_height, 400);
    assert!(p.final_bytes.unwrap() <= opts.cap_bytes);
    assert!(!p.bytes.is_empty());
    assert_eq!(p.mime.as_deref(), Some("image/jpeg"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn preview_of_a_corrupt_file_is_failed() {
    let dir = unique_dir("bad");
    let bad = dir.join("x.png");
    std::fs::write(&bad, b"not an image").unwrap();

    let p = preview(&bad, &Options::default());
    assert_eq!(p.kind, "failed");
    assert!(p.error.is_some());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn preview_of_a_large_image_is_a_close_estimate() {
    let dir = unique_dir("big");
    let src = dir.join("big.jpg");
    write_jpeg(&src, 2400, 1600); // larger than the preview's working dimension -> estimate
    let original = std::fs::metadata(&src).unwrap().len();
    let opts = Options {
        cap_bytes: original / 4,
        skip_if_under_cap: false,
        ..Options::default()
    };

    let p = preview(&src, &opts);
    assert_eq!(p.kind, "compressed");
    assert!(
        p.approx,
        "a large image preview should be flagged as an estimate"
    );
    let final_bytes = p.final_bytes.unwrap();
    assert!(final_bytes > 0);
    // The extrapolated estimate should be in the cap's ballpark, not off by an order of magnitude.
    assert!(
        final_bytes <= opts.cap_bytes * 2,
        "estimate {final_bytes} too far above cap {}",
        opts.cap_bytes
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn thumbnail_is_small_and_decodable() {
    let dir = unique_dir("thumb");
    let src = dir.join("t.jpg");
    write_jpeg(&src, 1600, 1200);

    let bytes = engine::thumbnail(&src, 96).unwrap();
    assert!(!bytes.is_empty());
    let img = engine::decode::decode(&bytes).unwrap();
    assert!(
        img.width().max(img.height()) <= 96,
        "thumbnail long edge {} exceeds 96",
        img.width().max(img.height())
    );

    std::fs::remove_dir_all(&dir).ok();
}
