use crate::error::EngineError;
use fast_image_resize::images::Image;
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::{DynamicImage, RgbaImage};

/// Downscale so the longest edge is at most `max_long_edge`. Never upscales.
pub fn downscale_to_long_edge(
    img: &DynamicImage,
    max_long_edge: u32,
) -> Result<DynamicImage, EngineError> {
    let (w, h) = (img.width(), img.height());
    let long = w.max(h);
    if max_long_edge == 0 || long <= max_long_edge {
        return Ok(img.clone());
    }
    let scale = f64::from(max_long_edge) / f64::from(long);
    let nw = ((f64::from(w) * scale).round() as u32).max(1);
    let nh = ((f64::from(h) * scale).round() as u32).max(1);
    resize_to(img, nw, nh)
}

/// Downscale by `factor` in `(0, 1)`. A `factor >= 1.0` returns a clone (never upscale).
pub fn downscale_by_factor(img: &DynamicImage, factor: f64) -> Result<DynamicImage, EngineError> {
    if factor >= 1.0 {
        return Ok(img.clone());
    }
    let nw = ((f64::from(img.width()) * factor).round() as u32).max(1);
    let nh = ((f64::from(img.height()) * factor).round() as u32).max(1);
    resize_to(img, nw, nh)
}

/// High-quality Lanczos3 resample via `fast_image_resize`. Everything is converted to RGBA8 so a
/// single code path covers all source pixel formats.
fn resize_to(img: &DynamicImage, nw: u32, nh: u32) -> Result<DynamicImage, EngineError> {
    let src_rgba = img.to_rgba8();
    let (w, h) = (src_rgba.width(), src_rgba.height());
    let src = Image::from_vec_u8(w, h, src_rgba.into_raw(), PixelType::U8x4)
        .map_err(|e| EngineError::Resize(e.to_string()))?;

    let mut dst = Image::new(nw, nh, PixelType::U8x4);
    let mut resizer = Resizer::new();
    let opts = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
    resizer
        .resize(&src, &mut dst, &opts)
        .map_err(|e| EngineError::Resize(e.to_string()))?;

    let rgba = RgbaImage::from_raw(nw, nh, dst.into_vec())
        .ok_or_else(|| EngineError::Resize("resized buffer size mismatch".to_string()))?;
    Ok(DynamicImage::ImageRgba8(rgba))
}
