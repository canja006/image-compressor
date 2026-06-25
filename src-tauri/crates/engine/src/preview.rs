//! Single-image preview: run the size search in memory and report what the user would get, plus
//! the encoded bytes for a live before/after readout. Nothing is written to disk.

use crate::batch::resolve_format_with_alpha;
use crate::crop::cover_crop_resize;
use crate::decode::decode;
use crate::encode::{encode, EncodeFormat};
use crate::error::EngineError;
use crate::model::{Options, ResizeMode};
use crate::resize::downscale_to_long_edge;
use crate::target::compress_to_target;
use image::DynamicImage;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub original_bytes: u64,
    pub source_width: u32,
    pub source_height: u32,
    pub has_alpha: bool,
    /// `"compressed"` | `"unreachable"` | `"failed"`.
    pub kind: String,
    pub final_bytes: Option<u64>,
    pub quality: Option<u8>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub downscaled: bool,
    /// True when the size is an estimate extrapolated from a downscaled search (large images).
    pub approx: bool,
    pub mime: Option<String>,
    pub error: Option<String>,
    /// SSIM and PSNR (dB) of the previewed output vs the source pixels — the B6 quality readout.
    /// `None` when not computable (e.g. AVIF, which has no pure-Rust decoder for measurement).
    pub ssim: Option<f64>,
    pub psnr: Option<f64>,
    /// Encoded preview bytes (only when `kind == "compressed"`). Skipped in serialization — the
    /// command base64-encodes these into a data URL for the webview instead of sending raw bytes.
    #[serde(skip)]
    pub bytes: Vec<u8>,
}

impl Preview {
    pub fn failed(original_bytes: u64, error: String) -> Self {
        Preview {
            original_bytes,
            source_width: 0,
            source_height: 0,
            has_alpha: false,
            kind: "failed".to_string(),
            final_bytes: None,
            quality: None,
            width: None,
            height: None,
            downscaled: false,
            approx: false,
            mime: None,
            error: Some(error),
            ssim: None,
            psnr: None,
            bytes: Vec::new(),
        }
    }
}

/// A decoded, downscaled image ready for the preview search. Cacheable so changing only the cap or
/// format doesn't re-decode the file — the expensive decode + resize happens once per image.
#[derive(Clone)]
pub struct PreviewSource {
    work: DynamicImage,
    base_width: u32,
    base_height: u32,
    work_pixels: u64,
    base_pixels: u64,
    has_alpha: bool,
    source_width: u32,
    source_height: u32,
}

/// Longest-edge cap for the downscaled copy the preview searches on. Small on purpose: the preview
/// panel displays it tiny, and fewer pixels mean each of the ~8 search encodes is fast. The size is
/// extrapolated back to the full resolution, so accuracy holds.
const PREVIEW_MAX_DIM: u32 = 720;

/// Decode `path`, size it per the resize mode, and downscale a working copy for a fast preview
/// search. The result is cacheable across cap/format changes (only the resize mode invalidates it).
pub fn prepare_source(path: &Path, resize: &ResizeMode) -> Result<PreviewSource, EngineError> {
    let raw = std::fs::read(path).map_err(|e| EngineError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let meta = crate::metadata::read_source_meta(&raw);
    let img = decode(&raw)?;
    // Bake EXIF orientation so the preview and its reported source dimensions are upright. sRGB is
    // not applied here: the source is cached by path + resize mode only, not the full options.
    let img = crate::metadata::apply_color_and_orientation(img, &meta, &Options::default());
    let source_width = img.width();
    let source_height = img.height();
    let has_alpha = img.color().has_alpha();

    // The intended output resolution: a fit-by-longest-edge bound, or the exact crop-to-fill target.
    let base = match *resize {
        ResizeMode::Fit { max_dimension } => match max_dimension {
            Some(maxd) => downscale_to_long_edge(&img, maxd)?,
            None => img,
        },
        ResizeMode::Exact {
            width,
            height,
            anchor,
            allow_upscale,
        } => cover_crop_resize(&img, width, height, anchor, allow_upscale)?,
    };
    let base_width = base.width();
    let base_height = base.height();
    let base_pixels = u64::from(base_width) * u64::from(base_height);

    let work = downscale_to_long_edge(&base, PREVIEW_MAX_DIM)?;
    let work_pixels = u64::from(work.width()) * u64::from(work.height());

    Ok(PreviewSource {
        work,
        base_width,
        base_height,
        work_pixels,
        base_pixels,
        has_alpha,
        source_width,
        source_height,
    })
}

/// Run the size search on an already-prepared source. Searches the downscaled copy and extrapolates
/// the size: lossy size scales ~linearly with pixel count at a fixed quality, so the quality found
/// matches the full-resolution result and the estimate is close (flagged `approx` when downscaled).
pub fn preview_from_source(
    source: &PreviewSource,
    original_bytes: u64,
    options: &Options,
) -> Preview {
    let fmt =
        resolve_format_with_alpha(options.output_format, source.has_alpha, options.background);
    let mut p = Preview {
        original_bytes,
        source_width: source.source_width,
        source_height: source.source_height,
        has_alpha: source.has_alpha,
        kind: "unreachable".to_string(),
        final_bytes: None,
        quality: None,
        width: None,
        height: None,
        downscaled: false,
        approx: false,
        mime: None,
        error: None,
        ssim: None,
        psnr: None,
        bytes: Vec::new(),
    };

    let approx = source.work_pixels < source.base_pixels;
    let ratio = (source.work_pixels as f64) / (source.base_pixels as f64); // in (0, 1]
    let search_cap = if approx {
        ((options.cap_bytes as f64) * ratio).round().max(1.0) as u64
    } else {
        options.cap_bytes
    };

    // Exact mode locks dimensions, so the preview search must vary quality only (matching the run).
    let allow_downscale = matches!(options.resize, ResizeMode::Fit { .. });
    match compress_to_target(
        &source.work,
        search_cap,
        fmt,
        options,
        allow_downscale,
        None,
    ) {
        Ok(Some(t)) => {
            // B6 readout: measure the previewed result against the (downscaled) source pixels.
            let (ssim_v, psnr_v) = measure_preview(&source.work, &t.bytes);
            p.ssim = ssim_v;
            p.psnr = psnr_v;
            let final_bytes = if approx {
                ((t.bytes.len() as f64) / ratio).round() as u64
            } else {
                t.bytes.len() as u64
            };
            p.kind = "compressed".to_string();
            p.final_bytes = Some(final_bytes);
            p.quality = t.quality;
            p.width = Some(source.base_width);
            p.height = Some(source.base_height);
            p.downscaled = t.downscaled;
            p.approx = approx;
            // AVIF may not render in older system WebViews, so show a web-safe JPEG stand-in of the
            // image for display. The size/quality readout above still reflects the real AVIF result.
            match fmt {
                EncodeFormat::Avif => {
                    let bg = options.background;
                    match encode(
                        &source.work,
                        EncodeFormat::Jpeg { background: bg },
                        Some(85),
                    ) {
                        Ok(jpeg) => {
                            p.mime = Some("image/jpeg".to_string());
                            p.bytes = jpeg;
                        }
                        Err(_) => {
                            p.mime = Some(fmt.mime().to_string());
                            p.bytes = t.bytes;
                        }
                    }
                }
                _ => {
                    p.mime = Some(fmt.mime().to_string());
                    p.bytes = t.bytes;
                }
            }
        }
        Ok(None) => {}
        Err(e) => return Preview::failed(original_bytes, e.to_string()),
    }
    p
}

/// SSIM/PSNR of an encoded preview buffer against the source pixels, or `(None, None)` when the
/// buffer can't be decoded (e.g. AVIF) or dimensions mismatch.
fn measure_preview(work: &DynamicImage, encoded: &[u8]) -> (Option<f64>, Option<f64>) {
    let Ok(decoded) = decode(encoded) else {
        return (None, None);
    };
    let a = work.to_rgb8();
    let b = decoded.to_rgb8();
    if a.dimensions() != b.dimensions() {
        return (None, None);
    }
    (
        Some(crate::metrics::ssim(&a, &b)),
        Some(crate::metrics::psnr(&a, &b)),
    )
}

/// Compress one image entirely in memory and return the result plus its encoded bytes.
pub fn preview(path: &Path, options: &Options) -> Preview {
    let original_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    match prepare_source(path, &options.resize) {
        Ok(source) => preview_from_source(&source, original_bytes, options),
        Err(e) => Preview::failed(original_bytes, e.to_string()),
    }
}

/// Decode `path` and return a small JPEG thumbnail (longest edge `max`) for the file list.
/// Transparent images are flattened onto white for the thumbnail.
pub fn thumbnail(path: &Path, max: u32) -> Result<Vec<u8>, EngineError> {
    let raw = std::fs::read(path).map_err(|e| EngineError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let meta = crate::metadata::read_source_meta(&raw);
    let img = decode(&raw)?;
    // Bake EXIF orientation so thumbnails aren't shown sideways.
    let img = crate::metadata::apply_color_and_orientation(img, &meta, &Options::default());
    let small = downscale_to_long_edge(&img, max.max(16))?;
    encode(
        &small,
        EncodeFormat::Jpeg {
            background: [255, 255, 255],
        },
        Some(70),
    )
}
