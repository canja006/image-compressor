//! Serde types shared across the Rust/TypeScript boundary. Field names are camelCase so the
//! frontend can use them directly; enums use discriminants the UI can switch on.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Output container the user wants. `Keep` picks PNG for images with alpha, JPEG otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Keep,
    Jpeg,
    Png,
    Avif,
}

/// Which edge a crop is anchored to on the axis being cropped when producing an exact size.
/// Applies to the cropped axis only: left/centre/right when trimming width, top/centre/bottom when
/// trimming height.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    /// Keep the top (cropping height) or left (cropping width) edge.
    Start,
    /// Centre the crop (default).
    #[default]
    Center,
    /// Keep the bottom (cropping height) or right (cropping width) edge.
    End,
}

/// How the engine sizes the output before the byte-cap search runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum ResizeMode {
    /// Preserve aspect ratio, bounding the longest edge. `max_dimension == None` means no resize.
    /// Never upscales; the cap search may still downscale dimensions to meet the cap.
    #[serde(rename_all = "camelCase")]
    Fit { max_dimension: Option<u32> },
    /// Produce EXACTLY `width` x `height` pixels via crop-to-fill (cover): scale and crop so the
    /// image fully covers the box — never adding borders. Dimensions are locked, so the cap search
    /// varies quality only and never downscales below the target (an unreachable cap stays
    /// unreachable rather than silently shrinking the image).
    #[serde(rename_all = "camelCase")]
    Exact {
        width: u32,
        height: u32,
        anchor: Anchor,
        allow_upscale: bool,
    },
}

impl Default for ResizeMode {
    fn default() -> Self {
        ResizeMode::Fit {
            max_dimension: None,
        }
    }
}

/// What to do when the computed output path already exists on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollisionPolicy {
    /// Append `-1`, `-2`, … until a free name is found.
    Suffix,
    /// Overwrite the existing file.
    Overwrite,
    /// Leave the existing file untouched and record the input as skipped.
    Skip,
}

/// Everything the engine needs to process a batch. Built by the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    /// The hard size cap, in bytes. Output is always `<= cap_bytes` when reachable.
    pub cap_bytes: u64,
    /// How the output is sized before the cap search: fit-by-longest-edge (the default) or an exact
    /// crop-to-fill target.
    pub resize: ResizeMode,
    pub output_format: OutputFormat,
    /// `None` writes next to each source file.
    pub output_dir: Option<PathBuf>,
    /// Appended to the file stem, e.g. `-compressed`.
    pub suffix: String,
    pub collision: CollisionPolicy,
    /// Re-encoding always drops metadata; kept for forward-compat / copy-as-is paths.
    pub strip_metadata: bool,
    /// If true, a source already `<= cap_bytes` is copied as-is instead of re-encoded.
    pub skip_if_under_cap: bool,
    /// Lower / upper bound of the JPEG quality binary search (1..=100).
    pub jpeg_quality_min: u8,
    pub jpeg_quality_max: u8,
    /// Floor on the longest edge during the downscale fallback; below this a cap is unreachable.
    pub min_long_edge: u32,
    /// Background used to flatten alpha when encoding an image with transparency to JPEG.
    pub background: [u8; 3],
}

impl Default for Options {
    fn default() -> Self {
        Self {
            cap_bytes: 500 * 1024,
            resize: ResizeMode::default(),
            output_format: OutputFormat::Jpeg,
            output_dir: None,
            suffix: "-compressed".to_string(),
            collision: CollisionPolicy::Suffix,
            strip_metadata: true,
            skip_if_under_cap: true,
            jpeg_quality_min: 10,
            jpeg_quality_max: 95,
            min_long_edge: 16,
            background: [255, 255, 255],
        }
    }
}

/// Per-file result. Exactly one variant is produced for every input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Outcome {
    /// Re-encoded and written under the cap.
    #[serde(rename_all = "camelCase")]
    Compressed {
        final_bytes: u64,
        /// `None` for formats without a lossy quality knob (e.g. PNG).
        quality: Option<u8>,
        width: u32,
        height: u32,
        /// True if dimensions had to shrink to meet the cap.
        downscaled: bool,
    },
    /// Source was already under the cap and copied as-is.
    SkippedUnderCap { bytes: u64 },
    /// Output already existed and the collision policy was `Skip`.
    SkippedCollision,
    /// Cap could not be met even at the dimension floor.
    Unreachable { reason: String },
    /// Decode/read/write failure — isolated to this file.
    Failed { reason: String },
    /// The job was cancelled before this file was processed.
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileResult {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub original_bytes: u64,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchSummary {
    pub results: Vec<FileResult>,
    pub cancelled: bool,
}

/// Emitted once per file as the batch progresses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub completed: usize,
    pub total: usize,
    pub result: FileResult,
}

/// A concrete, supported image file plus its on-disk size — what the UI lists for the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputFile {
    pub path: PathBuf,
    pub bytes: u64,
}

/// A file to process, optionally with its own size cap overriding the batch default.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchItem {
    pub path: PathBuf,
    pub cap_override: Option<u64>,
}

impl BatchItem {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            cap_override: None,
        }
    }
}

impl From<PathBuf> for BatchItem {
    fn from(path: PathBuf) -> Self {
        Self::new(path)
    }
}
