use engine::encode::{encode, EncodeFormat};
use engine::metadata::{apply_color_and_orientation, read_source_meta, SourceMeta};
use engine::model::{MetadataMode, Options};
use engine::{compress_batch, BatchItem, CollisionPolicy, Outcome, OutputFormat};
use image::{DynamicImage, RgbImage};
use img_parts::jpeg::Jpeg;
use img_parts::{Bytes, ImageICC};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

#[test]
fn orientation_6_swaps_width_and_height() {
    // 200 wide x 100 tall; EXIF orientation 6 = rotate 90 CW -> 100 x 200.
    let img = DynamicImage::ImageRgb8(RgbImage::new(200, 100));
    let meta = SourceMeta {
        orientation: Some(6),
        icc: None,
    };
    let out = apply_color_and_orientation(img, &meta, &Options::default());
    assert_eq!((out.width(), out.height()), (100, 200));
}

#[test]
fn orientation_1_is_identity() {
    let img = DynamicImage::ImageRgb8(RgbImage::new(200, 100));
    let meta = SourceMeta {
        orientation: Some(1),
        icc: None,
    };
    let out = apply_color_and_orientation(img, &meta, &Options::default());
    assert_eq!((out.width(), out.height()), (200, 100));
}

#[test]
fn read_source_meta_on_garbage_is_empty() {
    let meta = read_source_meta(b"not an image at all");
    assert!(meta.orientation.is_none());
    assert!(meta.icc.is_none());
}

#[test]
fn options_deserialize_when_new_fields_absent() {
    // Older frontends omit metadata/convertSrgb; serde(default) must fill them in.
    let mut v = serde_json::to_value(Options::default()).expect("serialize");
    let obj = v.as_object_mut().expect("object");
    obj.remove("metadata");
    obj.remove("convertSrgb");
    let o: Options = serde_json::from_value(v).expect("deserialize without new fields");
    assert_eq!(o.metadata, MetadataMode::StripAll);
    assert!(!o.convert_srgb);
}

/// Build a 64x64 JPEG carrying a (synthetic) ICC profile, written to `dir/name`.
fn jpeg_with_icc(dir: &Path, name: &str, icc: &[u8]) -> PathBuf {
    let mut img = RgbImage::new(64, 64);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = image::Rgb([((x ^ y) & 0xff) as u8, (y & 0xff) as u8, (x & 0xff) as u8]);
    }
    let base = encode(
        &DynamicImage::ImageRgb8(img),
        EncodeFormat::Jpeg {
            background: [255, 255, 255],
        },
        Some(90),
    )
    .expect("encode jpeg");
    let mut jpeg = Jpeg::from_bytes(Bytes::from(base)).expect("parse jpeg");
    jpeg.set_icc_profile(Some(Bytes::from(icc.to_vec())));
    let mut out = Vec::new();
    jpeg.encoder()
        .write_to(&mut out)
        .expect("write jpeg with icc");
    let path = dir.join(name);
    std::fs::write(&path, out).expect("write source");
    path
}

/// Read back the ICC profile of a compressed result's output file, if any.
fn output_icc(result: &engine::FileResult) -> Option<Vec<u8>> {
    let out = result.output.as_ref().expect("output path");
    let bytes = std::fs::read(out).expect("read output");
    Jpeg::from_bytes(Bytes::from(bytes))
        .ok()
        .and_then(|j| j.icc_profile())
        .map(|b| b.to_vec())
}

fn run_one(path: PathBuf, dir: &Path, metadata: MetadataMode) -> engine::FileResult {
    let options = Options {
        cap_bytes: 5_000_000,
        skip_if_under_cap: false, // force a re-encode so the metadata path runs
        collision: CollisionPolicy::Overwrite,
        output_dir: Some(dir.to_path_buf()),
        output_format: OutputFormat::Jpeg,
        metadata,
        ..Options::default()
    };
    let cancel = AtomicBool::new(false);
    let summary = compress_batch(&[BatchItem::new(path)], &options, &cancel, &|_p| {});
    summary.results.into_iter().next().expect("one result")
}

#[test]
fn strip_all_removes_icc_keep_all_preserves_it() {
    let dir = std::env::temp_dir().join(format!("ic_meta_e2e_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let icc = b"FAKE-ICC-PROFILE-FOR-ROUNDTRIP-TEST".to_vec();

    // Sanity: the source really carries the ICC, and read_source_meta surfaces it.
    let src = jpeg_with_icc(&dir, "src.jpg", &icc);
    let meta = read_source_meta(&std::fs::read(&src).unwrap());
    assert_eq!(
        meta.icc.as_deref(),
        Some(icc.as_slice()),
        "source should carry the ICC"
    );

    // StripAll (default) -> output has no ICC.
    let stripped = run_one(
        jpeg_with_icc(&dir, "strip.jpg", &icc),
        &dir,
        MetadataMode::StripAll,
    );
    assert!(matches!(stripped.outcome, Outcome::Compressed { .. }));
    assert!(
        output_icc(&stripped).is_none(),
        "StripAll must drop the ICC"
    );

    // KeepAll -> output preserves the ICC byte-for-byte.
    let kept = run_one(
        jpeg_with_icc(&dir, "keep.jpg", &icc),
        &dir,
        MetadataMode::KeepAll,
    );
    assert!(matches!(kept.outcome, Outcome::Compressed { .. }));
    assert_eq!(
        output_icc(&kept).as_deref(),
        Some(icc.as_slice()),
        "KeepAll must re-embed the ICC"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
