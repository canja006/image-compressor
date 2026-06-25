//! End-to-end tests for the `imgc` binary: they run the real compiled CLI against temp folders, so
//! they exercise exactly the engine code paths the GUI uses (B3 acceptance: a folder compressed to a
//! cap headlessly, with a non-zero exit on failure).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Synthesize a noisy (hence not-tiny) JPEG on disk so a small cap forces real compression.
fn write_sample_jpeg(path: &Path, w: u32, h: u32) {
    let mut img = image::RgbImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = image::Rgb([
            ((x.wrapping_mul(7)) ^ (y.wrapping_mul(13))) as u8,
            (x.wrapping_mul(3) & 0xff) as u8,
            (y & 0xff) as u8,
        ]);
    }
    image::DynamicImage::ImageRgb8(img)
        .save_with_format(path, image::ImageFormat::Jpeg)
        .expect("write sample jpeg");
}

/// A fresh, empty temp directory unique to this process + tag.
fn unique_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("imgc_cli_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn compresses_a_folder_to_a_cap_headlessly() {
    let src = unique_dir("ok_src");
    let out = unique_dir("ok_out");
    write_sample_jpeg(&src.join("a.jpg"), 1000, 1000);
    write_sample_jpeg(&src.join("b.jpg"), 800, 1200);

    let cap_bytes: u64 = 80 * 1024;
    let status = Command::new(env!("CARGO_BIN_EXE_imgc"))
        .arg(&src)
        .args(["--cap", "80k", "--format", "jpeg", "--quiet"])
        .arg("--out")
        .arg(&out)
        .status()
        .expect("run imgc");
    assert!(status.success(), "a reachable cap should exit success");

    let outputs: Vec<PathBuf> = std::fs::read_dir(&out)
        .expect("read output dir")
        .flatten()
        .map(|e| e.path())
        .collect();
    assert_eq!(outputs.len(), 2, "both images produce an output file");
    for p in &outputs {
        let size = std::fs::metadata(p).expect("stat output").len();
        assert!(
            size <= cap_bytes,
            "{} is {size} bytes, over the {cap_bytes}-byte cap",
            p.display()
        );
    }

    let _ = std::fs::remove_dir_all(&src);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn an_impossible_cap_exits_non_zero() {
    let src = unique_dir("bad_src");
    let out = unique_dir("bad_out");
    write_sample_jpeg(&src.join("a.jpg"), 600, 600);

    // 10 bytes is impossible for any real JPEG, so the file is Unreachable and the run must fail.
    let status = Command::new(env!("CARGO_BIN_EXE_imgc"))
        .arg(&src)
        .args(["--cap", "10", "--force-recompress", "--quiet"])
        .arg("--out")
        .arg(&out)
        .status()
        .expect("run imgc");
    assert!(!status.success(), "an impossible cap must exit non-zero");

    let _ = std::fs::remove_dir_all(&src);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn no_images_in_inputs_exits_non_zero() {
    let src = unique_dir("empty_src");
    // A non-image file: scan_inputs finds nothing supported.
    std::fs::write(src.join("notes.txt"), b"not an image").expect("write txt");

    let status = Command::new(env!("CARGO_BIN_EXE_imgc"))
        .arg(&src)
        .args(["--cap", "500k", "--quiet"])
        .status()
        .expect("run imgc");
    assert!(
        !status.success(),
        "no supported images should exit non-zero"
    );

    let _ = std::fs::remove_dir_all(&src);
}
