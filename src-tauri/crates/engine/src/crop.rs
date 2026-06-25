//! Crop-to-fill ("cover") cropping: produce an image at EXACT target dimensions with no borders.
//! The crop is computed in source space (one integer rectangle), then a single Lanczos resample
//! lands the result on the requested size. Pure geometry lives in [`cover_crop_rect`] so the
//! offsets are unit-testable without touching pixels.

use crate::error::EngineError;
use crate::model::Anchor;
use crate::resize::resize_exact;
use image::DynamicImage;

/// A crop rectangle in source pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CropRect {
    /// Left edge of the crop.
    pub x: u32,
    /// Top edge of the crop.
    pub y: u32,
    /// Crop width.
    pub w: u32,
    /// Crop height.
    pub h: u32,
}

/// Largest region of a `sw x sh` source that shares the aspect ratio of `tw x th`, positioned on
/// the cropped axis by `anchor` (the other axis is never cropped, so its offset is 0). The centre
/// offset uses integer division (floor), matching the worked example
/// `2560x1200 -> 1920x1080` which trims 213 px from the left.
pub fn cover_crop_rect(sw: u32, sh: u32, tw: u32, th: u32, anchor: Anchor) -> CropRect {
    if sw == 0 || sh == 0 || tw == 0 || th == 0 {
        return CropRect {
            x: 0,
            y: 0,
            w: sw,
            h: sh,
        };
    }

    let src_ar = f64::from(sw) / f64::from(sh);
    let target_ar = f64::from(tw) / f64::from(th);

    let (crop_w, crop_h) = if src_ar > target_ar {
        // Source is proportionally wider: keep full height, trim the sides.
        let w = (f64::from(sh) * target_ar).round() as u32;
        (w.clamp(1, sw), sh)
    } else {
        // Source is proportionally taller (or equal): keep full width, trim top/bottom.
        let h = (f64::from(sw) / target_ar).round() as u32;
        (sw, h.clamp(1, sh))
    };

    let x = match anchor {
        Anchor::Start => 0,
        Anchor::Center => (sw - crop_w) / 2,
        Anchor::End => sw - crop_w,
    };
    let y = match anchor {
        Anchor::Start => 0,
        Anchor::Center => (sh - crop_h) / 2,
        Anchor::End => sh - crop_h,
    };

    CropRect {
        x,
        y,
        w: crop_w,
        h: crop_h,
    }
}

/// Crop `img` to the aspect ratio of `tw x th` (anchored by `anchor`) and resample to exact pixels.
/// With `allow_upscale`, the output is exactly `tw x th`. Without it, the crop region is never
/// enlarged: the result is clamped to the largest same-aspect size the source supports (so a target
/// bigger than the source yields the crop region's own dimensions rather than an upscaled image).
pub fn cover_crop_resize(
    img: &DynamicImage,
    tw: u32,
    th: u32,
    anchor: Anchor,
    allow_upscale: bool,
) -> Result<DynamicImage, EngineError> {
    if tw == 0 || th == 0 {
        return Err(EngineError::Resize("exact target must be non-zero".into()));
    }

    let rect = cover_crop_rect(img.width(), img.height(), tw, th, anchor);
    let cropped = img.crop_imm(rect.x, rect.y, rect.w, rect.h);

    let (out_w, out_h) = if allow_upscale {
        (tw, th)
    } else {
        // Never enlarge the crop region: cap each axis to it. Since the crop region already has the
        // target aspect ratio, this preserves that ratio (a downscale stays exact; an upscale is
        // clamped back to the crop region's own size).
        (tw.min(rect.w), th.min(rect.h))
    };

    resize_exact(&cropped, out_w, out_h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, Rgb, RgbImage};

    fn img(w: u32, h: u32) -> DynamicImage {
        let mut buf = RgbImage::new(w, h);
        for (x, y, p) in buf.enumerate_pixels_mut() {
            *p = Rgb([((x ^ y) & 0xff) as u8, (y & 0xff) as u8, (x & 0xff) as u8]);
        }
        DynamicImage::ImageRgb8(buf)
    }

    #[test]
    fn centre_crop_matches_worked_example() {
        let rect = cover_crop_rect(2560, 1200, 1920, 1080, Anchor::Center);
        assert_eq!(
            rect,
            CropRect {
                x: 213,
                y: 0,
                w: 2133,
                h: 1200
            }
        );
    }

    #[test]
    fn exact_dims_for_wider_source() {
        let out = cover_crop_resize(&img(2000, 1000), 500, 500, Anchor::Center, true).unwrap();
        assert_eq!((out.width(), out.height()), (500, 500));
    }

    #[test]
    fn exact_dims_for_taller_source() {
        let out = cover_crop_resize(&img(1000, 2000), 500, 500, Anchor::Center, true).unwrap();
        assert_eq!((out.width(), out.height()), (500, 500));
    }

    #[test]
    fn anchor_start_and_end_shift_the_crop() {
        let (sw, sh, tw, th) = (2560, 1200, 1920, 1080);
        assert_eq!(cover_crop_rect(sw, sh, tw, th, Anchor::Start).x, 0);
        assert_eq!(cover_crop_rect(sw, sh, tw, th, Anchor::Center).x, 213);
        assert_eq!(cover_crop_rect(sw, sh, tw, th, Anchor::End).x, sw - 2133);
    }

    #[test]
    fn taller_source_crops_vertically() {
        let rect = cover_crop_rect(1000, 2000, 1000, 1000, Anchor::Center);
        assert_eq!((rect.w, rect.h, rect.y), (1000, 1000, 500));
    }

    #[test]
    fn no_upscale_clamps_to_crop_region() {
        // Target larger than the source on the covering axis: without upscaling, the output is the
        // crop region's own size (here the full square source), never enlarged to 4000x4000.
        let out = cover_crop_resize(&img(1000, 1000), 4000, 4000, Anchor::Center, false).unwrap();
        assert_eq!((out.width(), out.height()), (1000, 1000));
    }
}
