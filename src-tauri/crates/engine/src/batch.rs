//! Batch orchestration: fan out over files with rayon, isolate failures per file, honor a shared
//! cancel flag, and report progress as each file finishes.

use crate::decode::decode;
use crate::encode::EncodeFormat;
use crate::error::EngineError;
use crate::model::{FileResult, InputFile, Options, Outcome, OutputFormat, Progress};
use crate::naming::{resolve_output_path, Resolved};
use crate::resize::downscale_to_long_edge;
use crate::target::compress_to_target;
use image::DynamicImage;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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
    for p in paths {
        collect(p, &mut out, &mut seen);
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn collect(path: &Path, out: &mut Vec<InputFile>, seen: &mut HashSet<PathBuf>) {
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                collect(&entry.path(), out, seen);
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
    files: &[PathBuf],
    options: &Options,
    cancel: &AtomicBool,
    on_progress: &(dyn Fn(Progress) + Sync),
) -> crate::model::BatchSummary {
    let total = files.len();
    let completed = AtomicUsize::new(0);

    let results: Vec<FileResult> = files
        .par_iter()
        .map(|path| {
            let result = if cancel.load(Ordering::Relaxed) {
                FileResult {
                    input: path.clone(),
                    output: None,
                    original_bytes: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
                    outcome: Outcome::Cancelled,
                }
            } else {
                process_one(path, options)
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

/// Compress a single file end to end. Every failure mode is captured in the returned
/// `FileResult`; this function does not return `Result` because a bad file is a normal outcome.
fn process_one(path: &Path, options: &Options) -> FileResult {
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
    if options.skip_if_under_cap && original_bytes <= options.cap_bytes && original_bytes > 0 {
        match copy_as_is(path, options) {
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

    let fmt = resolve_format(options.output_format, &img, options.background);

    // Optional pre-downscale to the max dimension before the size search.
    let base = match options.max_dimension {
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

    match compress_to_target(&base, options.cap_bytes, fmt, options) {
        Ok(Some(target)) => match resolve_output_path(path, options, fmt.extension()) {
            Resolved::Path(out) => match std::fs::write(&out, &target.bytes) {
                Ok(()) => {
                    result.output = Some(out);
                    result.outcome = Outcome::Compressed {
                        final_bytes: target.bytes.len() as u64,
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
        },
        Ok(None) => {
            result.outcome = Outcome::Unreachable {
                reason: format!(
                    "cap of {} bytes not reachable above {}px",
                    options.cap_bytes, options.min_long_edge
                ),
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

fn resolve_format(of: OutputFormat, img: &DynamicImage, background: [u8; 3]) -> EncodeFormat {
    match of {
        OutputFormat::Jpeg => EncodeFormat::Jpeg { background },
        OutputFormat::Png => EncodeFormat::Png,
        OutputFormat::Keep => {
            if img.color().has_alpha() {
                EncodeFormat::Png
            } else {
                EncodeFormat::Jpeg { background }
            }
        }
    }
}

/// Copy a source that is already under the cap to its resolved output path, keeping the original
/// extension. Returns the destination, or `None` if a `Skip` collision policy declined it.
fn copy_as_is(path: &Path, options: &Options) -> Result<Option<PathBuf>, EngineError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("img");
    match resolve_output_path(path, options, ext) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::{encode, EncodeFormat};
    use crate::model::CollisionPolicy;
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

        let files = vec![good.clone(), corrupt.clone(), unreachable.clone()];
        let options = Options {
            cap_bytes: 10, // impossible for the real images; forces unreachable
            skip_if_under_cap: false,
            collision: CollisionPolicy::Overwrite,
            output_dir: Some(dir.clone()),
            ..Options::default()
        };
        let cancel = AtomicBool::new(false);
        let summary = compress_batch(&files, &options, &cancel, &|_p| {});

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
        let summary = compress_batch(&[f], &options, &cancel, &|_p| {});
        assert!(summary.cancelled);
        assert!(matches!(summary.results[0].outcome, Outcome::Cancelled));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
