//! EXIF orientation + ICC/sRGB color handling, all pure-Rust. Read a source file's metadata once,
//! bake its EXIF orientation into the pixels so every output is visually upright, optionally convert
//! pixels to sRGB via the source ICC profile, and (for non-strip modes) re-embed the ICC profile in
//! the encoded output. Phase 1 re-embeds the ICC profile only; full EXIF/GPS re-embedding is
//! deferred. Every function is best-effort and never panics on bad input.

use crate::model::{MetadataMode, Options};
use exif::{In, Reader, Tag};
use image::DynamicImage;
use img_parts::jpeg::Jpeg;
use img_parts::png::Png;
use img_parts::{Bytes, ImageICC};
use moxcms::{ColorProfile, Layout, TransformOptions};
use std::io::Cursor;

/// Metadata read once from a source file's ORIGINAL encoded bytes.
#[derive(Debug, Clone, Default)]
pub struct SourceMeta {
    /// EXIF orientation 1..=8, if present.
    pub orientation: Option<u32>,
    /// Raw ICC profile bytes, if the source carried one.
    pub icc: Option<Vec<u8>>,
}

/// Read EXIF orientation (kamadak-exif) and the ICC profile (img-parts) from the original bytes.
/// Missing/garbage metadata is normal — every branch falls back to `None`.
pub fn read_source_meta(original_bytes: &[u8]) -> SourceMeta {
    let orientation = Reader::new()
        .read_from_container(&mut Cursor::new(original_bytes))
        .ok()
        .and_then(|exif| {
            exif.get_field(Tag::Orientation, In::PRIMARY)
                .and_then(|f| f.value.get_uint(0))
        });

    let icc = Jpeg::from_bytes(Bytes::from(original_bytes.to_vec()))
        .ok()
        .and_then(|j| j.icc_profile())
        .or_else(|| {
            Png::from_bytes(Bytes::from(original_bytes.to_vec()))
                .ok()
                .and_then(|p| p.icc_profile())
        })
        .map(|b| b.to_vec());

    SourceMeta { orientation, icc }
}

/// Bake EXIF orientation into the pixels (so output is upright in every mode) and, if requested,
/// convert pixels from the source ICC profile to sRGB. Pure pixel work — touches no encoded metadata.
pub fn apply_color_and_orientation(
    img: DynamicImage,
    meta: &SourceMeta,
    opts: &Options,
) -> DynamicImage {
    let mut img = img;

    if let Some(o) = meta.orientation {
        if let Ok(o8) = u8::try_from(o) {
            if let Some(orient) = image::metadata::Orientation::from_exif(o8) {
                img.apply_orientation(orient);
            }
        }
    }

    if opts.convert_srgb {
        if let Some(icc) = meta.icc.as_ref() {
            img = to_srgb(img, icc);
        }
    }

    img
}

/// Transform `img` from the `icc` source profile to sRGB with the pure-Rust moxcms CMS. Alpha is
/// preserved (4-channel transform when present). Any failure returns `img` unchanged.
fn to_srgb(img: DynamicImage, icc: &[u8]) -> DynamicImage {
    let src_profile = match ColorProfile::new_from_slice(icc) {
        Ok(p) => p,
        Err(_) => return img,
    };
    let dst_profile = ColorProfile::new_srgb();
    let has_alpha = img.color().has_alpha();
    let layout = if has_alpha { Layout::Rgba } else { Layout::Rgb };

    let transform = match src_profile.create_transform_8bit(
        layout,
        &dst_profile,
        layout,
        TransformOptions::default(),
    ) {
        Ok(t) => t,
        Err(_) => return img,
    };

    let (w, h) = (img.width(), img.height());
    let src_data = if has_alpha {
        img.to_rgba8().into_raw()
    } else {
        img.to_rgb8().into_raw()
    };
    let mut dst = vec![0u8; src_data.len()];
    if transform.transform(&src_data, &mut dst).is_err() {
        return img;
    }

    let rebuilt = if has_alpha {
        image::RgbaImage::from_raw(w, h, dst).map(DynamicImage::ImageRgba8)
    } else {
        image::RgbImage::from_raw(w, h, dst).map(DynamicImage::ImageRgb8)
    };
    rebuilt.unwrap_or(img)
}

/// Re-embed metadata into already-encoded output bytes per the mode. `StripAll` (the default) is a
/// passthrough — the freshly-encoded bytes carry no metadata. The keep modes re-embed the ICC profile
/// so color management survives (unless pixels were already converted to sRGB, in which case no ICC is
/// embedded). Full EXIF/GPS re-embedding is deferred. Best-effort: any failure returns `encoded`.
pub fn mux_metadata(encoded: Vec<u8>, meta: &SourceMeta, opts: &Options, fmt_ext: &str) -> Vec<u8> {
    if opts.metadata == MetadataMode::StripAll {
        return encoded;
    }

    // Pixels already in sRGB after conversion -> embedding the source ICC would mis-tag them.
    let icc_to_embed: Option<Bytes> = if opts.convert_srgb {
        None
    } else {
        meta.icc.clone().map(Bytes::from)
    };

    match fmt_ext {
        "jpg" | "jpeg" => match Jpeg::from_bytes(Bytes::from(encoded.clone())) {
            Ok(mut j) => {
                j.set_icc_profile(icc_to_embed);
                let mut out = Vec::new();
                if j.encoder().write_to(&mut out).is_ok() {
                    out
                } else {
                    encoded
                }
            }
            Err(_) => encoded,
        },
        "png" => match Png::from_bytes(Bytes::from(encoded.clone())) {
            Ok(mut p) => {
                p.set_icc_profile(icc_to_embed);
                let mut out = Vec::new();
                if p.encoder().write_to(&mut out).is_ok() {
                    out
                } else {
                    encoded
                }
            }
            Err(_) => encoded,
        },
        _ => encoded,
    }
}

/// How the skip/copy path (an already-under-cap file) should emit its output so the result still
/// honors the metadata policy. A verbatim `std::fs::copy` would leak EXIF/GPS the user asked to
/// strip, so [`plan_copy`] picks one of these instead.
pub enum CopyPlan {
    /// Copy the source bytes through unchanged — `KeepAll`, where retaining everything is the intent.
    Verbatim,
    /// Write these rewritten bytes: identical pixels, metadata stripped per the mode (lossless).
    Stripped(Vec<u8>),
    /// A lossless copy can't honor the policy (a rotated JPEG whose orientation must be baked into
    /// the pixels, a non-JPEG container, or a parse failure). The caller should re-encode instead,
    /// which reliably drops all metadata and bakes orientation.
    Reencode,
}

/// Decide how the skip/copy path should write `original` so the output matches `mode`. An upright
/// JPEG (no EXIF orientation, or orientation 1) gets a lossless segment strip; everything else falls
/// back to a re-encode so no metadata can leak. `KeepAll` is always a verbatim copy.
///
/// This exists because "already under the cap, copy as-is" must not become a privacy hole: with the
/// default `StripAll`, an under-cap photo would otherwise be copied with its GPS intact.
pub fn plan_copy(original: &[u8], mode: MetadataMode, ext: &str) -> CopyPlan {
    if mode == MetadataMode::KeepAll {
        return CopyPlan::Verbatim;
    }
    // A rotated source needs its orientation baked into the pixels to stay upright once the EXIF
    // orientation tag is dropped; only a re-encode can do that.
    let meta = read_source_meta(original);
    if matches!(meta.orientation, Some(2..=8)) {
        return CopyPlan::Reencode;
    }
    match ext.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => match strip_jpeg_metadata(original, mode) {
            Some(bytes) => CopyPlan::Stripped(bytes),
            None => CopyPlan::Reencode,
        },
        // PNG re-encoding (oxipng) is lossless and already strips metadata, so routing PNG — and any
        // other container (webp/tiff/avif) — through the normal path costs no quality and cannot leak.
        _ => CopyPlan::Reencode,
    }
}

/// Losslessly remove privacy-bearing metadata from an encoded JPEG without re-compressing the pixels:
/// every APP1 (EXIF incl. GPS, and XMP), APP13 (Photoshop/IPTC) and COM (comment) segment is dropped;
/// `StripAll` additionally drops the ICC profile, while the keep / strip-GPS modes retain it. Returns
/// `None` on any parse/encode failure so the caller falls back to a full re-encode.
fn strip_jpeg_metadata(original: &[u8], mode: MetadataMode) -> Option<Vec<u8>> {
    const APP1: u8 = 0xE1; // EXIF (incl. GPS) + XMP
    const APP13: u8 = 0xED; // Photoshop / IPTC
    const COM: u8 = 0xFE; // free-text comment
    let mut jpeg = Jpeg::from_bytes(Bytes::from(original.to_vec())).ok()?;
    jpeg.remove_segments_by_marker(APP1);
    jpeg.remove_segments_by_marker(APP13);
    jpeg.remove_segments_by_marker(COM);
    if mode == MetadataMode::StripAll {
        jpeg.set_icc_profile(None);
    }
    let mut out = Vec::new();
    jpeg.encoder().write_to(&mut out).ok()?;
    Some(out)
}
