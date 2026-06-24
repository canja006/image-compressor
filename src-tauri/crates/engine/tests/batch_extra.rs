//! End-to-end batch behavior against real files on disk: directory scanning, a real compression
//! that writes an output under the cap, and the skip-if-under-cap copy path.

use engine::encode::{encode, EncodeFormat};
use engine::model::{BatchItem, CollisionPolicy, Options, Outcome, OutputFormat};
use engine::{compress_batch, is_supported, scan_inputs};
use image::{DynamicImage, Rgb, RgbImage};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

fn unique_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ic_batchx_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_jpeg(path: &Path, w: u32, h: u32, quality: u8) {
    let mut img = RgbImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = Rgb([((x ^ y) & 0xff) as u8, (y & 0xff) as u8, (x & 0xff) as u8]);
    }
    let bytes = encode(
        &DynamicImage::ImageRgb8(img),
        EncodeFormat::Jpeg {
            background: [255, 255, 255],
        },
        Some(quality),
    )
    .unwrap();
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn is_supported_is_case_insensitive() {
    assert!(is_supported(Path::new("a.JPG")));
    assert!(is_supported(Path::new("b.png")));
    assert!(is_supported(Path::new("c.WEBP")));
    assert!(!is_supported(Path::new("d.txt")));
    assert!(!is_supported(Path::new("noext")));
}

#[test]
fn scan_inputs_walks_filters_dedups_and_sorts() {
    let dir = unique_dir("scan");
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    write_jpeg(&dir.join("b.jpg"), 32, 32, 80);
    write_jpeg(&sub.join("a.jpg"), 32, 32, 80);
    std::fs::write(dir.join("notes.txt"), b"ignore me").unwrap();

    // Pass the directory plus an explicit duplicate of b.jpg to confirm de-duplication.
    let found = scan_inputs(&[dir.clone(), dir.join("b.jpg")]);
    let paths: Vec<PathBuf> = found.iter().map(|f| f.path.clone()).collect();
    assert_eq!(paths, vec![dir.join("b.jpg"), sub.join("a.jpg")]);
    assert!(
        found.iter().all(|f| f.bytes > 0),
        "sizes should be populated"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn real_batch_writes_an_output_under_the_cap() {
    let dir = unique_dir("real");
    let src = dir.join("big.jpg");
    write_jpeg(&src, 1200, 900, 95);
    let cap = (std::fs::metadata(&src).unwrap().len() / 3).max(2_000);

    let opts = Options {
        cap_bytes: cap,
        output_format: OutputFormat::Jpeg,
        output_dir: Some(dir.clone()),
        skip_if_under_cap: false,
        collision: CollisionPolicy::Overwrite,
        ..Options::default()
    };
    let cancel = AtomicBool::new(false);
    let summary = compress_batch(&[BatchItem::new(src)], &opts, &cancel, &|_p| {});
    let result = &summary.results[0];

    match &result.outcome {
        Outcome::Compressed { final_bytes, .. } => {
            assert!(*final_bytes <= cap, "{final_bytes} exceeds cap {cap}");
            let out = result.output.clone().expect("an output path");
            assert!(out.exists(), "the output file should exist");
            assert!(std::fs::metadata(&out).unwrap().len() <= cap);
        }
        other => panic!("expected Compressed, got {other:?}"),
    }

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn skip_if_under_cap_copies_the_source() {
    let dir = unique_dir("skip");
    let src = dir.join("small.jpg");
    write_jpeg(&src, 16, 16, 50);
    let original_len = std::fs::metadata(&src).unwrap().len();

    let opts = Options {
        cap_bytes: 10_000_000, // far above the tiny source
        output_dir: Some(dir.clone()),
        skip_if_under_cap: true,
        collision: CollisionPolicy::Suffix,
        ..Options::default()
    };
    let cancel = AtomicBool::new(false);
    let summary = compress_batch(&[BatchItem::new(src)], &opts, &cancel, &|_p| {});

    assert!(matches!(
        summary.results[0].outcome,
        Outcome::SkippedUnderCap { .. }
    ));
    let out = summary.results[0].output.clone().expect("a copied output");
    assert_eq!(
        std::fs::metadata(&out).unwrap().len(),
        original_len,
        "an as-is copy keeps the byte size"
    );

    std::fs::remove_dir_all(&dir).ok();
}
