use crate::error::EngineError;
use crate::model::Options;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{DynamicImage, ExtendedColorType, ImageEncoder, RgbImage};

/// A concrete encoder target. JPEG carries the background used to flatten alpha; PNG is lossless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeFormat {
    Jpeg { background: [u8; 3] },
    Png,
}

impl EncodeFormat {
    /// File extension for the output path.
    pub fn extension(&self) -> &'static str {
        match self {
            EncodeFormat::Jpeg { .. } => "jpg",
            EncodeFormat::Png => "png",
        }
    }

    /// The `(min, max)` quality range for the size search, or `None` for formats without a
    /// lossy quality knob. `max` is clamped to be `>= min` so the search range is always valid.
    pub fn quality_range(&self, opts: &Options) -> Option<(u8, u8)> {
        match self {
            EncodeFormat::Jpeg { .. } => {
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
    }
}

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
    Ok(buf)
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
