use crate::error::EngineError;
use crate::model::Options;
#[cfg(not(feature = "mozjpeg"))]
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{DynamicImage, ExtendedColorType, ImageEncoder, RgbImage};
use ravif::{Encoder, Img};
use rgb::FromSlice;

/// A concrete encoder target. JPEG carries the background used to flatten alpha; PNG and AVIF are
/// alpha-preserving (PNG lossless, AVIF lossy with a quality knob).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeFormat {
    Jpeg { background: [u8; 3] },
    Png,
    Avif,
}

impl EncodeFormat {
    /// File extension for the output path.
    pub fn extension(&self) -> &'static str {
        match self {
            EncodeFormat::Jpeg { .. } => "jpg",
            EncodeFormat::Png => "png",
            EncodeFormat::Avif => "avif",
        }
    }

    /// MIME type, used when building a preview data URL.
    pub fn mime(&self) -> &'static str {
        match self {
            EncodeFormat::Jpeg { .. } => "image/jpeg",
            EncodeFormat::Png => "image/png",
            EncodeFormat::Avif => "image/avif",
        }
    }

    /// The `(min, max)` quality range for the size search, or `None` for formats without a
    /// lossy quality knob. `max` is clamped to be `>= min` so the search range is always valid.
    /// JPEG and AVIF share the user's configured quality bounds.
    pub fn quality_range(&self, opts: &Options) -> Option<(u8, u8)> {
        match self {
            EncodeFormat::Jpeg { .. } | EncodeFormat::Avif => {
                let lo = opts.jpeg_quality_min.clamp(1, 100);
                let hi = opts.jpeg_quality_max.clamp(lo, 100);
                Some((lo, hi))
            }
            EncodeFormat::Png => None,
        }
    }
}

/// Encode `img` to an in-memory buffer. `quality` is ignored for formats without a quality knob.
pub fn encode(
    img: &DynamicImage,
    fmt: EncodeFormat,
    quality: Option<u8>,
) -> Result<Vec<u8>, EngineError> {
    match fmt {
        EncodeFormat::Jpeg { background } => encode_jpeg(img, quality.unwrap_or(75), background),
        EncodeFormat::Png => encode_png(img),
        EncodeFormat::Avif => encode_avif(img, quality.unwrap_or(60)),
    }
}

/// Encode to AVIF with the pure-Rust `ravif`/rav1e encoder. Alpha is preserved. Speed 8 keeps the
/// per-step encode fast enough for the size search (the spec accepts AVIF being slower overall).
fn encode_avif(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, EngineError> {
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    let raw: &[u8] = rgba.as_raw();
    let encoded = Encoder::new()
        .with_quality(f32::from(quality.clamp(1, 100)))
        .with_speed(8)
        .encode_rgba(Img::new(raw.as_rgba(), w, h))
        .map_err(|e| EngineError::Encode(e.to_string()))?;
    Ok(encoded.avif_file)
}

/// Pure-Rust JPEG encode via the `image` crate. The default (feature-OFF) path — needs no C toolchain.
#[cfg(not(feature = "mozjpeg"))]
fn encode_jpeg(
    img: &DynamicImage,
    quality: u8,
    background: [u8; 3],
) -> Result<Vec<u8>, EngineError> {
    let rgb = flatten_to_rgb(img, background);
    let mut buf = Vec::new();
    {
        let mut enc = JpegEncoder::new_with_quality(&mut buf, quality.clamp(1, 100));
        enc.encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ExtendedColorType::Rgb8,
        )
        .map_err(|e| EngineError::Encode(e.to_string()))?;
    }
    Ok(buf)
}

/// mozjpeg (trellis-quantized) JPEG encode — smaller files at equal quality. Compiled only with
/// `--features mozjpeg`; statically links libjpeg-turbo/mozjpeg at build time. The encoded pixels are
/// the SAME flattened RGB8 the pure-Rust path uses, so the size search stays encoder-agnostic.
#[cfg(feature = "mozjpeg")]
fn encode_jpeg(
    img: &DynamicImage,
    quality: u8,
    background: [u8; 3],
) -> Result<Vec<u8>, EngineError> {
    let rgb = flatten_to_rgb(img, background);
    let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    comp.set_size(rgb.width() as usize, rgb.height() as usize);
    comp.set_quality(f32::from(quality.clamp(1, 100)));
    let mut started = comp
        .start_compress(Vec::new())
        .map_err(|e| EngineError::Encode(e.to_string()))?;
    started
        .write_scanlines(rgb.as_raw())
        .map_err(|e| EngineError::Encode(e.to_string()))?;
    let buf = started
        .finish()
        .map_err(|e| EngineError::Encode(e.to_string()))?;
    Ok(buf)
}

fn encode_png(img: &DynamicImage) -> Result<Vec<u8>, EngineError> {
    let mut buf = Vec::new();
    let enc = PngEncoder::new_with_quality(&mut buf, CompressionType::Best, FilterType::Adaptive);
    if img.color().has_alpha() {
        let rgba = img.to_rgba8();
        enc.write_image(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|e| EngineError::Encode(e.to_string()))?;
    } else {
        let rgb = img.to_rgb8();
        enc.write_image(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ExtendedColorType::Rgb8,
        )
        .map_err(|e| EngineError::Encode(e.to_string()))?;
    }
    // Lossless post-optimization with oxipng — shrinks the PNG further with no pixel change, so a
    // PNG cap is reachable at a higher resolution before any downscaling. Best-effort: if it fails,
    // the already-valid PNG is returned unchanged.
    match oxipng::optimize_from_memory(&buf, &oxipng::Options::from_preset(2)) {
        Ok(optimized) => Ok(optimized),
        Err(_) => Ok(buf),
    }
}

/// Drop alpha by compositing over a solid `background`. JPEG has no alpha channel, so a
/// transparent source must be flattened or the transparent areas would render as black.
fn flatten_to_rgb(img: &DynamicImage, background: [u8; 3]) -> RgbImage {
    if !img.color().has_alpha() {
        return img.to_rgb8();
    }
    let rgba = img.to_rgba8();
    let mut out = RgbImage::new(rgba.width(), rgba.height());
    for (x, y, px) in rgba.enumerate_pixels() {
        let a = u32::from(px[3]);
        let inv = 255 - a;
        let blend = |c: u8, bg: u8| ((u32::from(c) * a + u32::from(bg) * inv) / 255) as u8;
        out.put_pixel(
            x,
            y,
            image::Rgb([
                blend(px[0], background[0]),
                blend(px[1], background[1]),
                blend(px[2], background[2]),
            ]),
        );
    }
    out
}
