//! Batch orchestration: fan out over files with rayon, isolate failures per file, honor a shared
//! cancel flag, and report progress as each file finishes.

use crate::crop::cover_crop_resize;
use crate::decode::decode;
use crate::encode::EncodeFormat;
use crate::error::EngineError;
use crate::model::{
    BatchItem, FileResult, InputFile, MetadataMode, Options, Outcome, OutputFormat, Progress,
    ResizeMode,
};
use crate::naming::{output_base_name, resolve_output_path_with_base, NameInfo, Resolved};
use crate::resize::downscale_to_long_edge;
use crate::target::compress_to_target;
use image::DynamicImage;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering};

/// File extensions the engine will attempt to decode.
pub const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "tif", "tiff"];

/// True if `path` has a supported image extension (case-insensitive).
pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Expand a mix of files and directories into concrete supported image files with their sizes.
/// Directories are walked recursively; duplicates are removed; order is deterministic (sorted).
pub fn scan_inputs(paths: &[PathBuf]) -> Vec<InputFile> {
    let mut out: Vec<InputFile> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut visited_dirs: HashSet<PathBuf> = HashSet::new();
    for p in paths {
        collect(p, &mut out, &mut seen, &mut visited_dirs);
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn collect(
    path: &Path,
    out: &mut Vec<InputFile>,
    seen: &mut HashSet<PathBuf>,
    visited_dirs: &mut HashSet<PathBuf>,
) {
    if path.is_dir() {
        // Guard against symlink loops (a folder linking back to an ancestor would otherwise recurse
        // forever): only descend into each real directory once, keyed by its canonical path.
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if !visited_dirs.insert(canonical) {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                collect(&entry.path(), out, seen, visited_dirs);
            }
        }
    } else if path.is_file() && is_supported(path) && seen.insert(path.to_path_buf()) {
        let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        out.push(InputFile {
            path: path.to_path_buf(),
            bytes,
        });
    }
}

/// Process every file in parallel (rayon, capped to the core count by the global pool).
/// `cancel` is checked before each file; `on_progress` is called once per file as it completes.
/// One corrupt or unreachable file never aborts the batch.
pub fn compress_batch(
    items: &[BatchItem],
    options: &Options,
    cancel: &AtomicBool,
    on_progress: &(dyn Fn(Progress) + Sync),
) -> crate::model::BatchSummary {
    let total = items.len();
    let completed = AtomicUsize::new(0);
    // Warm-start hint shared across the batch: the last successful JPEG quality (0 = none). Similar
    // images converge to similar quality, so seeding the binary search narrows it. Relaxed races are
    // harmless — it only ever shrinks the search range; the result stays optimal regardless.
    let quality_hint = AtomicU16::new(0);
    // One date stamp for the whole batch, for the `{date}` rename token.
    let date = today_ymd();

    let results: Vec<FileResult> = items
        .par_iter()
        .enumerate()
        .map(|(idx, item)| {
            let path = item.path.as_path();
            let cap = item.cap_override.unwrap_or(options.cap_bytes);
            let result = if cancel.load(Ordering::Relaxed) {
                FileResult {
                    input: item.path.clone(),
                    output: None,
                    original_bytes: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
                    outcome: Outcome::Cancelled,
                }
            } else {
                process_one(path, options, cap, &quality_hint, idx + 1, &date)
            };
            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            on_progress(Progress {
                completed: done,
                total,
                result: result.clone(),
            });
            result
        })
        .collect();

    crate::model::BatchSummary {
        cancelled: cancel.load(Ordering::Relaxed),
        results,
    }
}

/// Compress a single file end to end against an effective `cap` (the per-file override or the
/// batch default). Every failure mode is captured in the returned `FileResult`; this function does
/// not return `Result` because a bad file is a normal outcome.
fn process_one(
    path: &Path,
    options: &Options,
    cap: u64,
    quality_hint: &AtomicU16,
    seq: usize,
    date: &str,
) -> FileResult {
    let original_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let mut result = FileResult {
        input: path.to_path_buf(),
        output: None,
        original_bytes,
        outcome: Outcome::Failed {
            reason: "unprocessed".to_string(),
        },
    };

    // Already under the cap: copy as-is rather than re-encode (configurable).
    if options.skip_if_under_cap && original_bytes <= cap && original_bytes > 0 {
        match copy_as_is(path, options, seq, date) {
            Ok(Some(out)) => {
                result.output = Some(out);
                result.outcome = Outcome::SkippedUnderCap {
                    bytes: original_bytes,
                };
            }
            Ok(None) => result.outcome = Outcome::SkippedCollision,
            Err(e) => {
                result.outcome = Outcome::Failed {
                    reason: e.to_string(),
                }
            }
        }
        return result;
    }

    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            result.outcome = Outcome::Failed {
                reason: format!("read: {e}"),
            };
            return result;
        }
    };
    // Read EXIF/ICC from the original bytes before they are dropped; used to bake orientation, run
    // the optional sRGB conversion, and (for keep modes) re-embed the ICC profile after encoding.
    let meta = crate::metadata::read_source_meta(&bytes);
    let img = match decode(&bytes) {
        Ok(i) => i,
        Err(e) => {
            result.outcome = Outcome::Failed {
                reason: e.to_string(),
            };
            return result;
        }
    };
    drop(bytes);
    // Bake EXIF orientation into the pixels (every output is upright) and optionally convert to sRGB.
    let img = crate::metadata::apply_color_and_orientation(img, &meta, options);

    let fmt = resolve_format(options.output_format, &img, options.background);

    // Size the image per the resize mode before the cap search. Fit may pre-downscale by the longest
    // edge and lets the search shrink further; Exact crops to fill the locked target, after which the
    // search varies quality only (`allow_downscale = false`).
    let (base, allow_downscale) = match options.resize {
        ResizeMode::Fit { max_dimension } => {
            let sized = match max_dimension {
                Some(maxd) => match downscale_to_long_edge(&img, maxd) {
                    Ok(i) => i,
                    Err(e) => {
                        result.outcome = Outcome::Failed {
                            reason: e.to_string(),
                        };
                        return result;
                    }
                },
                None => img,
            };
            (sized, true)
        }
        ResizeMode::Exact {
            width,
            height,
            anchor,
            allow_upscale,
        } => match cover_crop_resize(&img, width, height, anchor, allow_upscale) {
            Ok(i) => (i, false),
            Err(e) => {
                result.outcome = Outcome::Failed {
                    reason: e.to_string(),
                };
                return result;
            }
        },
    };

    // Reserve room for any re-embedded ICC profile so the final file still fits the cap. Only the
    // non-strip metadata modes embed bytes, and only when the pixels were not converted to sRGB
    // (a converted image embeds no ICC). The default StripAll mode reserves nothing.
    let metadata_reserve: u64 =
        if options.metadata != MetadataMode::StripAll && !options.convert_srgb {
            match (&meta.icc, fmt.extension()) {
                (Some(icc), "jpg" | "png") => icc.len() as u64 + 24,
                _ => 0,
            }
        } else {
            0
        };
    let effective_cap = cap.saturating_sub(metadata_reserve);

    let hint = match quality_hint.load(Ordering::Relaxed) {
        0 => None,
        q => u8::try_from(q).ok(),
    };
    match compress_to_target(&base, effective_cap, fmt, options, allow_downscale, hint) {
        Ok(Some(target)) => {
            // Feed this file's quality back as the warm-start hint for the next similar image.
            if let Some(q) = target.quality {
                quality_hint.store(u16::from(q), Ordering::Relaxed);
            }
            // Re-embed metadata per mode (passthrough for StripAll), then write the muxed bytes.
            let final_bytes =
                crate::metadata::mux_metadata(target.bytes, &meta, options, fmt.extension());
            let info = NameInfo {
                seq,
                width: target.width,
                height: target.height,
                date,
            };
            let base = output_base_name(path, options, &info);
            match resolve_output_path_with_base(path, options, fmt.extension(), &base) {
                Resolved::Path(out) => match std::fs::write(&out, &final_bytes) {
                    Ok(()) => {
                        result.output = Some(out);
                        result.outcome = Outcome::Compressed {
                            final_bytes: final_bytes.len() as u64,
                            quality: target.quality,
                            width: target.width,
                            height: target.height,
                            downscaled: target.downscaled,
                        };
                    }
                    Err(e) => {
                        result.outcome = Outcome::Failed {
                            reason: format!("write: {e}"),
                        };
                    }
                },
                Resolved::SkipCollision => result.outcome = Outcome::SkippedCollision,
            }
        }
        Ok(None) => {
            result.outcome = Outcome::Unreachable {
                reason: match options.resize {
                    ResizeMode::Exact { width, height, .. } => format!(
                        "cap of {cap} bytes not reachable at the locked {width}×{height} size"
                    ),
                    ResizeMode::Fit { .. } => format!(
                        "cap of {} bytes not reachable above {}px",
                        cap, options.min_long_edge
                    ),
                },
            };
        }
        Err(e) => {
            result.outcome = Outcome::Failed {
                reason: e.to_string(),
            }
        }
    }

    result
}

pub(crate) fn resolve_format(
    of: OutputFormat,
    img: &DynamicImage,
    background: [u8; 3],
) -> EncodeFormat {
    resolve_format_with_alpha(of, img.color().has_alpha(), background)
}

pub(crate) fn resolve_format_with_alpha(
    of: OutputFormat,
    has_alpha: bool,
    background: [u8; 3],
) -> EncodeFormat {
    match of {
        OutputFormat::Jpeg => EncodeFormat::Jpeg { background },
        OutputFormat::Png => EncodeFormat::Png,
        OutputFormat::Avif => EncodeFormat::Avif,
        OutputFormat::Keep => {
            if has_alpha {
                EncodeFormat::Png
            } else {
                EncodeFormat::Jpeg { background }
            }
        }
    }
}

/// Copy a source that is already under the cap to its resolved output path, keeping the original
/// extension. Returns the destination, or `None` if a `Skip` collision policy declined it. Honors a
/// rename pattern; `{w}`/`{h}` come from a cheap header read since the copy isn't decoded otherwise.
fn copy_as_is(
    path: &Path,
    options: &Options,
    seq: usize,
    date: &str,
) -> Result<Option<PathBuf>, EngineError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("img");
    let (width, height) = if options.rename_pattern.is_some() {
        image::image_dimensions(path).unwrap_or((0, 0))
    } else {
        (0, 0)
    };
    let info = NameInfo {
        seq,
        width,
        height,
        date,
    };
    let base = output_base_name(path, options, &info);
    match resolve_output_path_with_base(path, options, ext, &base) {
        Resolved::Path(out) => {
            if out == path {
                return Ok(Some(out)); // output would be the source itself; nothing to copy
            }
            std::fs::copy(path, &out).map_err(|e| EngineError::Io {
                path: out.clone(),
                source: e,
            })?;
            Ok(Some(out))
        }
        Resolved::SkipCollision => Ok(None),
    }
}

/// Current UTC date as `YYYY-MM-DD` for the `{date}` rename token. Falls back to the epoch on a clock
/// error. Pure date math — no `chrono` dependency.
fn today_ymd() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d) = ymd_from_days((secs / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Civil (year, month, day) from a day count since 1970-01-01 (Howard Hinnant's `civil_from_days`).
fn ymd_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::{encode, EncodeFormat};
    use crate::model::{Anchor, CollisionPolicy};
    use image::{Rgb, RgbImage};
    use std::sync::atomic::AtomicBool;

    fn write_test_jpeg(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
        let mut img = RgbImage::new(w, h);
        for (x, y, p) in img.enumerate_pixels_mut() {
            *p = Rgb([((x ^ y) & 0xff) as u8, (y & 0xff) as u8, (x & 0xff) as u8]);
        }
        let bytes = encode(
            &image::DynamicImage::ImageRgb8(img),
            EncodeFormat::Jpeg {
                background: [255, 255, 255],
            },
            Some(95),
        )
        .unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn mixed_batch_isolates_corrupt_and_unreachable_files() {
        let dir = std::env::temp_dir().join(format!("ic_batch_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let good = write_test_jpeg(&dir, "good.jpg", 800, 800);
        // A corrupt file: a .png extension with non-image bytes.
        let corrupt = dir.join("corrupt.png");
        std::fs::write(&corrupt, b"this is definitely not a png").unwrap();
        // A file whose cap is impossible (10 bytes) -> unreachable.
        let unreachable = write_test_jpeg(&dir, "unreachable.jpg", 600, 600);

        let items: Vec<BatchItem> = vec![good.clone(), corrupt.clone(), unreachable.clone()]
            .into_iter()
            .map(BatchItem::from)
            .collect();
        let options = Options {
            cap_bytes: 10, // impossible for the real images; forces unreachable
            skip_if_under_cap: false,
            collision: CollisionPolicy::Overwrite,
            output_dir: Some(dir.clone()),
            ..Options::default()
        };
        let cancel = AtomicBool::new(false);
        let summary = compress_batch(&items, &options, &cancel, &|_p| {});

        assert_eq!(summary.results.len(), 3, "every file is reported");
        assert!(!summary.cancelled);

        let by_input = |p: &PathBuf| {
            summary
                .results
                .iter()
                .find(|r| &r.input == p)
                .map(|r| r.outcome.clone())
                .unwrap()
        };
        assert!(matches!(by_input(&corrupt), Outcome::Failed { .. }));
        assert!(matches!(
            by_input(&unreachable),
            Outcome::Unreachable { .. }
        ));
        assert!(matches!(by_input(&good), Outcome::Unreachable { .. }));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancelled_flag_yields_cancelled_outcomes() {
        let dir = std::env::temp_dir().join(format!("ic_cancel_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = write_test_jpeg(&dir, "a.jpg", 200, 200);

        let options = Options {
            output_dir: Some(dir.clone()),
            ..Options::default()
        };
        let cancel = AtomicBool::new(true); // pre-cancelled
        let summary = compress_batch(&[BatchItem::new(f)], &options, &cancel, &|_p| {});
        assert!(summary.cancelled);
        assert!(matches!(summary.results[0].outcome, Outcome::Cancelled));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn per_item_cap_override_takes_precedence_over_the_batch_cap() {
        let dir = std::env::temp_dir().join(format!("ic_override_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = write_test_jpeg(&dir, "x.jpg", 500, 500);

        // Batch cap is huge (would easily succeed), but this item's 10-byte override is impossible.
        let options = Options {
            cap_bytes: 10_000_000,
            skip_if_under_cap: false,
            collision: CollisionPolicy::Overwrite,
            output_dir: Some(dir.clone()),
            ..Options::default()
        };
        let items = vec![BatchItem {
            path: f,
            cap_override: Some(10),
        }];
        let cancel = AtomicBool::new(false);
        let summary = compress_batch(&items, &options, &cancel, &|_p| {});
        assert!(
            matches!(summary.results[0].outcome, Outcome::Unreachable { .. }),
            "the per-item override should force unreachable, not the lenient batch cap"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn exact_mode_outputs_locked_dimensions() {
        let dir = std::env::temp_dir().join(format!("ic_exact_ok_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = write_test_jpeg(&dir, "wide.jpg", 800, 600);

        let options = Options {
            cap_bytes: 5_000_000, // generous: reachable at the locked size
            skip_if_under_cap: false,
            collision: CollisionPolicy::Overwrite,
            output_dir: Some(dir.clone()),
            resize: ResizeMode::Exact {
                width: 400,
                height: 400,
                anchor: Anchor::Center,
                allow_upscale: true,
            },
            ..Options::default()
        };
        let cancel = AtomicBool::new(false);
        let summary = compress_batch(&[BatchItem::new(f)], &options, &cancel, &|_p| {});

        match &summary.results[0].outcome {
            Outcome::Compressed {
                width,
                height,
                downscaled,
                final_bytes,
                ..
            } => {
                assert_eq!((*width, *height), (400, 400), "exact size must be honored");
                assert!(!downscaled, "exact mode never downscales dimensions");
                assert!(*final_bytes <= options.cap_bytes);
            }
            other => panic!("expected Compressed at 400x400, got {other:?}"),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn exact_mode_unreachable_cap_keeps_locked_size() {
        let dir = std::env::temp_dir().join(format!("ic_exact_unreach_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = write_test_jpeg(&dir, "wide.jpg", 800, 600);

        // 10 bytes is impossible for any real JPEG; with dimensions locked the engine must report
        // Unreachable rather than fall back to a smaller image.
        let options = Options {
            cap_bytes: 10,
            skip_if_under_cap: false,
            collision: CollisionPolicy::Overwrite,
            output_dir: Some(dir.clone()),
            resize: ResizeMode::Exact {
                width: 400,
                height: 400,
                anchor: Anchor::Center,
                allow_upscale: true,
            },
            ..Options::default()
        };
        let cancel = AtomicBool::new(false);
        let summary = compress_batch(&[BatchItem::new(f)], &options, &cancel, &|_p| {});
        assert!(
            matches!(summary.results[0].outcome, Outcome::Unreachable { .. }),
            "an impossible cap at locked dimensions must be Unreachable, not a shrunk image"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[cfg(unix)]
    fn scan_inputs_terminates_on_symlink_cycles() {
        use std::os::unix::fs::symlink;
        let dir = std::env::temp_dir().join(format!("ic_symlink_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let img = write_test_jpeg(&dir, "a.jpg", 32, 32);
        // A directory symlink pointing back to its own parent forms a cycle.
        let _ = symlink(&dir, dir.join("loop"));

        // Must terminate (not recurse forever) and still find the real image.
        let found = scan_inputs(std::slice::from_ref(&dir));
        assert!(
            found.iter().any(|f| f.path == img),
            "the real image should be found"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn civil_date_from_days_is_correct() {
        assert_eq!(ymd_from_days(0), (1970, 1, 1));
        assert_eq!(ymd_from_days(31), (1970, 2, 1));
        assert_eq!(ymd_from_days(59), (1970, 3, 1)); // 1970 is not a leap year
        assert_eq!(ymd_from_days(365), (1971, 1, 1));
    }

    #[test]
    fn rename_pattern_drives_the_output_filename() {
        let dir = std::env::temp_dir().join(format!("ic_rename_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = write_test_jpeg(&dir, "original.jpg", 320, 240);

        let options = Options {
            cap_bytes: 5_000_000,
            skip_if_under_cap: false,
            collision: CollisionPolicy::Overwrite,
            output_dir: Some(dir.clone()),
            rename_pattern: Some("{name}-{seq:000}-{w}x{h}".to_string()),
            ..Options::default()
        };
        let cancel = AtomicBool::new(false);
        let summary = compress_batch(&[BatchItem::new(f)], &options, &cancel, &|_p| {});

        let out = summary.results[0].output.as_ref().expect("an output path");
        let stem = out.file_stem().and_then(|s| s.to_str()).unwrap();
        // seq is 1 (single file); dimensions are the encoded 320x240.
        assert_eq!(
            stem, "original-001-320x240",
            "the rename pattern should drive the output name"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
