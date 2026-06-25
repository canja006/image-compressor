//! The core: compress an already-decoded image so the encoded result fits a byte cap, returning
//! the best quality (largest file) still under the cap. Implements spec section 2 exactly:
//! quality binary search, then a dimension-downscale fallback, then `Unreachable`.

use crate::encode::{encode, EncodeFormat};
use crate::error::EngineError;
use crate::model::Options;
use crate::resize::downscale_by_factor;
use image::DynamicImage;

#[derive(Debug, Clone)]
pub struct TargetResult {
    /// The encoded winning buffer — written to disk exactly once by the caller.
    pub bytes: Vec<u8>,
    /// `Some(q)` for quality-search formats (JPEG); `None` for lossless formats (PNG).
    pub quality: Option<u8>,
    pub width: u32,
    pub height: u32,
    pub downscaled: bool,
    /// SSIM of the chosen output vs the source pixels, computed only when a perceptual floor is set
    /// (and the format is decodable for measurement). `None` otherwise.
    pub ssim: Option<f64>,
}

/// Returns `Ok(Some(result))` with the largest file that still fits `cap_bytes`, `Ok(None)` if the
/// cap is unreachable even after downscaling to the floor, or `Err` only on a genuine encode
/// failure (never on "too big" — that is data, not an error).
///
/// `allow_downscale` controls the dimension fallback: in fit mode it is `true`, so a too-large image
/// shrinks dimensions to meet the cap. In exact-size mode it is `false` — dimensions are locked to
/// the caller's target, only quality varies, and a cap that cannot be met at those dimensions is
/// reported `Unreachable` rather than silently delivering a smaller image.
/// `quality_hint` is an optional warm start from a previous similar image in the batch: the search
/// probes it first to narrow the binary-search range (a pure speedup — the result is still optimal).
///
/// `opts.perceptual_floor` (SSIM) adds a fidelity guard: if the cap-fitting result scores below the
/// floor and dimensions are not locked, the search trades resolution (downscales a step) to recover
/// fidelity rather than ship a visually-poor image. The floor applies to decodable lossy formats
/// (JPEG); lossless PNG is exact, and AVIF cannot be decoded for measurement so the floor is skipped.
pub fn compress_to_target(
    img: &DynamicImage,
    cap_bytes: u64,
    fmt: EncodeFormat,
    opts: &Options,
    allow_downscale: bool,
    quality_hint: Option<u8>,
) -> Result<Option<TargetResult>, EngineError> {
    let range = fmt.quality_range(opts);
    let q_lo = range.map(|(lo, _)| lo);
    let mut factor = 1.0_f64;

    loop {
        let work = downscale_by_factor(img, factor)?;
        let (w, h) = (work.width(), work.height());

        // Smallest file this size can produce (lowest quality, or the single lossless encode).
        // If even this exceeds the cap, the only lever left is dimensions.
        let smallest = encode(&work, fmt, q_lo)?;

        if smallest.len() as u64 > cap_bytes {
            // Dimensions are locked (exact mode): the cap simply can't be met here.
            if !allow_downscale {
                return Ok(None);
            }
            let long = w.max(h);
            if long <= opts.min_long_edge {
                return Ok(None); // can't shrink further and still over cap -> unreachable
            }
            factor = next_factor(
                factor,
                long,
                smallest.len() as u64,
                cap_bytes,
                opts.min_long_edge,
            );
            continue;
        }

        // The smallest encode fits. For lossless formats that single encode is the answer.
        let Some((lo, hi)) = range else {
            return Ok(Some(TargetResult {
                bytes: smallest,
                quality: None,
                width: w,
                height: h,
                downscaled: factor < 1.0,
                ssim: None,
            }));
        };

        // Binary-search the quality upward for the largest file still within the cap. An optional
        // warm-start hint replaces the first probe to shrink the range when batch images are similar.
        let mut best_q = lo;
        let mut best_bytes = smallest;
        let (mut low, mut high) = (lo + 1, hi);
        if let Some(hq) = quality_hint {
            if hq > lo && hq <= hi {
                let candidate = encode(&work, fmt, Some(hq))?;
                if candidate.len() as u64 <= cap_bytes {
                    best_q = hq;
                    best_bytes = candidate;
                    low = hq + 1;
                } else {
                    high = hq - 1;
                }
            }
        }
        while low <= high {
            let mid = low + (high - low) / 2;
            let candidate = encode(&work, fmt, Some(mid))?;
            if candidate.len() as u64 <= cap_bytes {
                best_q = mid;
                best_bytes = candidate;
                low = mid + 1;
            } else {
                high = mid - 1;
            }
        }

        // Perceptual floor: if the result is below the SSIM threshold, trade resolution for fidelity
        // (fit mode only). Measured only for decodable formats; AVIF/None leave the floor unenforced.
        let mut ssim_val = None;
        if let Some(floor) = opts.perceptual_floor {
            ssim_val = ssim_of(&work, &best_bytes);
            if let Some(sv) = ssim_val {
                if sv < floor && allow_downscale && w.max(h) > opts.min_long_edge {
                    factor = floor_shrink(factor, w.max(h), opts.min_long_edge);
                    continue;
                }
            }
        }

        return Ok(Some(TargetResult {
            bytes: best_bytes,
            quality: Some(best_q),
            width: w,
            height: h,
            downscaled: factor < 1.0,
            ssim: ssim_val,
        }));
    }
}

/// SSIM of an encoded buffer against the source pixels, or `None` if the buffer can't be decoded
/// (e.g. AVIF, for which no pure-Rust decoder is wired) or dimensions mismatch.
fn ssim_of(work: &DynamicImage, encoded: &[u8]) -> Option<f64> {
    let decoded = crate::decode::decode(encoded).ok()?;
    let a = work.to_rgb8();
    let b = decoded.to_rgb8();
    if a.dimensions() != b.dimensions() {
        return None;
    }
    Some(crate::metrics::ssim(&a, &b))
}

/// Shrink the cumulative scale factor one step for the perceptual-floor downscale, never jumping
/// below the dimension floor and always making progress (terminates the loop).
fn floor_shrink(factor: f64, long: u32, floor_edge: u32) -> f64 {
    let projected = (f64::from(long) * 0.85).round() as u32;
    let next = if projected < floor_edge {
        factor * (f64::from(floor_edge) / f64::from(long))
    } else {
        factor * 0.85
    };
    if next >= factor {
        factor * 0.85
    } else {
        next
    }
}

/// Pick the next cumulative scale factor when even the lowest quality overshoots the cap.
/// File size scales roughly with pixel count, so cutting the long edge by `sqrt(cap/size)`
/// targets the cap; clamp the per-step shrink to avoid huge jumps or glacial progress, and never
/// jump below the dimension floor in a single step.
fn next_factor(factor: f64, long: u32, size: u64, cap: u64, floor: u32) -> f64 {
    let ratio = cap as f64 / size as f64;
    let step = ratio.sqrt().clamp(0.5, 0.85);
    let projected_long = (f64::from(long) * step).round() as u32;
    let next = if projected_long < floor {
        factor * (f64::from(floor) / f64::from(long))
    } else {
        factor * step
    };
    // Numerical safety: always make progress.
    if next >= factor {
        factor * 0.85
    } else {
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::encode;
    use crate::model::Options;
    use image::{DynamicImage, Rgb, RgbImage};

    /// A deterministic, moderately detailed image. A flat color would compress to almost nothing
    /// and wouldn't exercise the quality search; this XOR/gradient pattern gives JPEG sizes that
    /// grow monotonically with quality.
    fn test_image(w: u32, h: u32) -> DynamicImage {
        let mut img = RgbImage::new(w, h);
        for (x, y, p) in img.enumerate_pixels_mut() {
            let r = ((x ^ y) & 0xff) as u8;
            let g = (x.wrapping_add(y) & 0xff) as u8;
            let b = ((x.wrapping_mul(2) ^ y) & 0xff) as u8;
            *p = Rgb([r, g, b]);
        }
        DynamicImage::ImageRgb8(img)
    }

    fn jpeg() -> EncodeFormat {
        EncodeFormat::Jpeg {
            background: [255, 255, 255],
        }
    }

    #[test]
    fn output_is_under_cap_when_reachable() {
        let img = test_image(512, 512);
        let opts = Options::default();
        let lo = encode(&img, jpeg(), Some(opts.jpeg_quality_min))
            .unwrap()
            .len() as u64;
        let hi = encode(&img, jpeg(), Some(opts.jpeg_quality_max))
            .unwrap()
            .len() as u64;
        let cap = (lo + hi) / 2;

        let res = compress_to_target(&img, cap, jpeg(), &opts, true, None)
            .unwrap()
            .expect("cap should be reachable");
        assert!(
            res.bytes.len() as u64 <= cap,
            "output {} exceeded cap {cap}",
            res.bytes.len()
        );
    }

    #[test]
    fn search_returns_best_quality_under_cap() {
        let img = test_image(512, 512);
        let opts = Options::default();
        let lo = encode(&img, jpeg(), Some(opts.jpeg_quality_min))
            .unwrap()
            .len() as u64;
        let hi = encode(&img, jpeg(), Some(opts.jpeg_quality_max))
            .unwrap()
            .len() as u64;
        assert!(hi > lo, "size must grow with quality for this test image");
        // A midpoint cap forces an interior solution (no downscale, q below max).
        let cap = (lo + hi) / 2;

        let res = compress_to_target(&img, cap, jpeg(), &opts, true, None)
            .unwrap()
            .expect("cap should be reachable");
        let q = res.quality.expect("jpeg result has a quality");

        assert!(
            !res.downscaled,
            "midpoint cap should not require downscaling"
        );
        assert!(res.bytes.len() as u64 <= cap);
        // The defining property: one quality step higher would exceed the cap.
        assert!(
            q < opts.jpeg_quality_max,
            "expected an interior quality, got {q}"
        );
        let higher = encode(&img, jpeg(), Some(q + 1)).unwrap().len() as u64;
        assert!(
            higher > cap,
            "quality {} gave {higher} which still fits cap {cap}; search was not optimal",
            q + 1
        );
    }

    #[test]
    fn tiny_cap_triggers_downscale_and_meets_cap() {
        let img = test_image(2000, 2000);
        let opts = Options::default();
        // Quarter of the smallest full-resolution encode: only dimension downscaling can hit this.
        let full_min = encode(&img, jpeg(), Some(opts.jpeg_quality_min))
            .unwrap()
            .len() as u64;
        let cap = full_min / 4;

        let res = compress_to_target(&img, cap, jpeg(), &opts, true, None)
            .unwrap()
            .expect("cap should be reachable via downscale");
        assert!(res.downscaled, "expected dimension downscaling");
        assert!(
            res.bytes.len() as u64 <= cap,
            "output {} exceeded cap {cap}",
            res.bytes.len()
        );
        assert!(
            res.width < 2000 && res.height < 2000,
            "dimensions should have shrunk"
        );
    }

    #[test]
    fn impossible_cap_is_unreachable_not_panic() {
        let img = test_image(1024, 1024);
        let opts = Options::default();
        // 10 bytes is smaller than any real JPEG header — unreachable at any size.
        let res = compress_to_target(&img, 10, jpeg(), &opts, true, None).unwrap();
        assert!(
            res.is_none(),
            "an impossible cap must be Unreachable, not a panic or a fit"
        );
    }

    #[test]
    fn lossless_png_path_has_no_quality() {
        let img = test_image(256, 256);
        let opts = Options::default();
        let big_cap = encode(&img, EncodeFormat::Png, None).unwrap().len() as u64 + 1;
        let res = compress_to_target(&img, big_cap, EncodeFormat::Png, &opts, true, None)
            .unwrap()
            .expect("png under a generous cap is reachable");
        assert_eq!(res.quality, None);
        assert!(res.bytes.len() as u64 <= big_cap);
    }

    #[test]
    fn locked_dimensions_never_downscale() {
        // A cap only dimension-downscaling could meet: in fit mode it succeeds by shrinking, but
        // with dimensions locked it must report unreachable instead of silently shrinking.
        let img = test_image(2000, 2000);
        let opts = Options::default();
        let cap = encode(&img, jpeg(), Some(opts.jpeg_quality_min))
            .unwrap()
            .len() as u64
            / 4;

        let fit = compress_to_target(&img, cap, jpeg(), &opts, true, None)
            .unwrap()
            .expect("fit mode reaches the cap by downscaling");
        assert!(fit.downscaled, "fit mode should have downscaled");

        let locked = compress_to_target(&img, cap, jpeg(), &opts, false, None).unwrap();
        assert!(
            locked.is_none(),
            "locked dimensions must be unreachable, not a downscaled fit"
        );
    }

    #[test]
    fn locked_dimensions_keep_size_when_reachable() {
        // A reachable cap with dimensions locked: quality varies but width/height are untouched.
        let img = test_image(512, 512);
        let opts = Options::default();
        let hi = encode(&img, jpeg(), Some(opts.jpeg_quality_max))
            .unwrap()
            .len() as u64;
        let cap = hi + 1; // comfortably reachable at full quality

        let res = compress_to_target(&img, cap, jpeg(), &opts, false, None)
            .unwrap()
            .expect("reachable cap at locked dimensions");
        assert!(!res.downscaled, "locked dimensions never downscale");
        assert_eq!((res.width, res.height), (512, 512));
        assert!(res.bytes.len() as u64 <= cap);
    }

    #[test]
    fn perceptual_floor_trades_resolution_for_fidelity() {
        // A detailed (high-frequency) image whose low-quality full-res JPEG scores poorly on SSIM.
        let img = test_image(1024, 1024);
        let base = Options::default();
        let lo_bytes = encode(&img, jpeg(), Some(base.jpeg_quality_min))
            .unwrap()
            .len() as u64;
        // A cap just above the smallest full-res encode: reachable at full resolution, but only at
        // low quality (low fidelity).
        let cap = lo_bytes + lo_bytes / 10;

        // Without a floor, the cap is met at full resolution (no downscale).
        let no_floor = compress_to_target(&img, cap, jpeg(), &base, true, None)
            .unwrap()
            .expect("reachable without a floor");
        assert!(
            !no_floor.downscaled,
            "without a floor the cap is met at full resolution"
        );
        assert!(
            no_floor.ssim.is_none(),
            "ssim only computed when a floor is set"
        );

        // A high floor forces the search to downscale to recover fidelity.
        let opts = Options {
            perceptual_floor: Some(0.99),
            ..Options::default()
        };
        let floored = compress_to_target(&img, cap, jpeg(), &opts, true, None)
            .unwrap()
            .expect("still reachable with a floor (best-effort at the dimension floor)");
        assert!(
            floored.downscaled,
            "a high perceptual floor should trade resolution for fidelity"
        );
        assert!(floored.ssim.is_some(), "floor runs compute an SSIM");
        assert!(
            floored.width < no_floor.width || floored.height < no_floor.height,
            "floored output should be smaller in at least one dimension"
        );
        assert!(
            floored.bytes.len() as u64 <= cap,
            "floor never breaks the cap"
        );
    }
}
