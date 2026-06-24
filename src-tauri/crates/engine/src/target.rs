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
}

/// Returns `Ok(Some(result))` with the largest file that still fits `cap_bytes`, `Ok(None)` if the
/// cap is unreachable even after downscaling to the floor, or `Err` only on a genuine encode
/// failure (never on "too big" — that is data, not an error).
pub fn compress_to_target(
    img: &DynamicImage,
    cap_bytes: u64,
    fmt: EncodeFormat,
    opts: &Options,
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
            }));
        };

        // Binary-search the quality upward for the largest file still within the cap.
        let mut best_q = lo;
        let mut best_bytes = smallest;
        let (mut low, mut high) = (lo + 1, hi);
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

        return Ok(Some(TargetResult {
            bytes: best_bytes,
            quality: Some(best_q),
            width: w,
            height: h,
            downscaled: factor < 1.0,
        }));
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

        let res = compress_to_target(&img, cap, jpeg(), &opts)
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

        let res = compress_to_target(&img, cap, jpeg(), &opts)
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

        let res = compress_to_target(&img, cap, jpeg(), &opts)
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
        let res = compress_to_target(&img, 10, jpeg(), &opts).unwrap();
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
        let res = compress_to_target(&img, big_cap, EncodeFormat::Png, &opts)
            .unwrap()
            .expect("png under a generous cap is reachable");
        assert_eq!(res.quality, None);
        assert!(res.bytes.len() as u64 <= big_cap);
    }
}
