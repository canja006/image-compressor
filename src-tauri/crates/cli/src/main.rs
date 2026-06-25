//! `imgc` — the headless image compressor. Depends on the pure-Rust `engine` crate only (never the
//! Tauri app), so the GUI and the CLI share one compression implementation. Reads files/folders,
//! compresses each to a per-file byte cap, and exits non-zero if any file failed or was unreachable.

use clap::{Parser, ValueEnum};
use engine::{
    compress_batch, scan_inputs, Anchor, BatchItem, CollisionPolicy, MetadataMode, Options,
    Outcome, OutputFormat, Progress, ResizeMode,
};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

#[derive(Parser)]
#[command(
    name = "imgc",
    version,
    about = "Compress images to a target file size."
)]
struct Cli {
    /// One or more image files or folders (folders are walked recursively).
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    /// Target size per file, e.g. 500k, 2MB, 750000 (base 1024).
    #[arg(long, value_parser = parse_size)]
    cap: u64,

    /// Output container.
    #[arg(long, default_value = "keep")]
    format: FormatArg,

    /// Bound the longest edge to this many pixels (fit mode). Ignored with --exact.
    #[arg(long)]
    max_dimension: Option<u32>,

    /// Crop to exactly WxH pixels, e.g. 1920x1080 (exact mode).
    #[arg(long, value_parser = parse_dims, conflicts_with = "max_dimension")]
    exact: Option<(u32, u32)>,

    /// Crop anchor for --exact.
    #[arg(long, default_value = "center")]
    anchor: AnchorArg,

    /// Allow upscaling to reach the --exact size.
    #[arg(long)]
    allow_upscale: bool,

    /// Write outputs here (default: next to each source file).
    #[arg(long)]
    out: Option<PathBuf>,

    /// Suffix appended to each output's file stem.
    #[arg(long, default_value = "-compressed")]
    suffix: String,

    /// Which EXIF/ICC metadata to re-embed (orientation is always baked into the pixels).
    #[arg(long, default_value = "strip-all")]
    metadata: MetadataArg,

    /// Convert pixels to sRGB (via the source ICC) before encoding.
    #[arg(long)]
    srgb: bool,

    /// Output-name pattern, e.g. "{name}-{seq:000}" (tokens: {name} {seq} {seq:000} {date} {w} {h}).
    #[arg(long)]
    rename: Option<String>,

    /// What to do when an output path already exists.
    #[arg(long, default_value = "suffix")]
    collision: CollisionArg,

    /// Lower bound of the JPEG quality search (1-100).
    #[arg(long, default_value_t = 10)]
    quality_min: u8,

    /// Upper bound of the JPEG quality search (1-100).
    #[arg(long, default_value_t = 95)]
    quality_max: u8,

    /// Perceptual quality floor as a percentage SSIM (50-100); trades resolution to stay sharp.
    #[arg(long)]
    floor: Option<u8>,

    /// Re-encode files even if they are already under the cap (default: copy them as-is).
    #[arg(long)]
    force_recompress: bool,

    /// Suppress the per-file progress lines (the final summary still prints).
    #[arg(long)]
    quiet: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
    Keep,
    Jpeg,
    Png,
    Avif,
}

impl From<FormatArg> for OutputFormat {
    fn from(arg: FormatArg) -> Self {
        match arg {
            FormatArg::Keep => OutputFormat::Keep,
            FormatArg::Jpeg => OutputFormat::Jpeg,
            FormatArg::Png => OutputFormat::Png,
            FormatArg::Avif => OutputFormat::Avif,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum AnchorArg {
    Start,
    Center,
    End,
}

impl From<AnchorArg> for Anchor {
    fn from(arg: AnchorArg) -> Self {
        match arg {
            AnchorArg::Start => Anchor::Start,
            AnchorArg::Center => Anchor::Center,
            AnchorArg::End => Anchor::End,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum MetadataArg {
    StripAll,
    KeepAll,
    KeepOrientationIcc,
    StripGps,
}

impl From<MetadataArg> for MetadataMode {
    fn from(arg: MetadataArg) -> Self {
        match arg {
            MetadataArg::StripAll => MetadataMode::StripAll,
            MetadataArg::KeepAll => MetadataMode::KeepAll,
            MetadataArg::KeepOrientationIcc => MetadataMode::KeepOrientationIcc,
            MetadataArg::StripGps => MetadataMode::StripGps,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum CollisionArg {
    Suffix,
    Overwrite,
    Skip,
}

impl From<CollisionArg> for CollisionPolicy {
    fn from(arg: CollisionArg) -> Self {
        match arg {
            CollisionArg::Suffix => CollisionPolicy::Suffix,
            CollisionArg::Overwrite => CollisionPolicy::Overwrite,
            CollisionArg::Skip => CollisionPolicy::Skip,
        }
    }
}

/// Parse a human size into bytes (base 1024). Accepts a bare number (bytes) or a `k`/`m`/`g` suffix
/// (optionally `b`/`kb`/`kib`, case-insensitive); the numeric part may be a decimal.
fn parse_size(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty size".to_string());
    }
    let split = s
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit() && *c != '.')
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    let (num_str, suffix) = s.split_at(split);
    let num: f64 = num_str
        .parse()
        .map_err(|_| format!("invalid number: {num_str}"))?;
    if !num.is_finite() || num <= 0.0 {
        return Err("size must be positive and finite".to_string());
    }
    let multiplier: u64 = match suffix.to_ascii_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kb" | "kib" => 1024,
        "m" | "mb" | "mib" => 1024 * 1024,
        "g" | "gb" | "gib" => 1024 * 1024 * 1024,
        other => return Err(format!("unknown size suffix: {other}")),
    };
    Ok((num * multiplier as f64).round() as u64)
}

/// Parse `WxH` into a `(width, height)` pair; both must be >= 1.
fn parse_dims(s: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = s.split(['x', 'X']).collect();
    if parts.len() != 2 {
        return Err("dimensions must look like WxH, e.g. 1920x1080".to_string());
    }
    let width: u32 = parts[0].parse().map_err(|_| "invalid width".to_string())?;
    let height: u32 = parts[1].parse().map_err(|_| "invalid height".to_string())?;
    if width == 0 || height == 0 {
        return Err("dimensions must be greater than zero".to_string());
    }
    Ok((width, height))
}

/// The file name as a lossy string, falling back to the full path display.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Compact human size for status text (base 1024).
fn human_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

/// One-line description of a per-file outcome.
fn describe(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Compressed {
            final_bytes,
            width,
            height,
            ..
        } => format!(
            "compressed → {} ({width}×{height})",
            human_bytes(*final_bytes)
        ),
        Outcome::SkippedUnderCap { bytes } => {
            format!("skipped (under cap, {})", human_bytes(*bytes))
        }
        Outcome::SkippedCollision => "skipped (output exists)".to_string(),
        Outcome::Unreachable { reason } => format!("unreachable: {reason}"),
        Outcome::Failed { reason } => format!("failed: {reason}"),
        Outcome::Cancelled => "cancelled".to_string(),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let perceptual_floor = match cli.floor {
        Some(pct) => {
            if !(50..=100).contains(&pct) {
                eprintln!("--floor must be between 50 and 100");
                return ExitCode::FAILURE;
            }
            Some(f64::from(pct) / 100.0)
        }
        None => None,
    };

    let resize = match cli.exact {
        Some((width, height)) => ResizeMode::Exact {
            width,
            height,
            anchor: cli.anchor.into(),
            allow_upscale: cli.allow_upscale,
        },
        None => ResizeMode::Fit {
            max_dimension: cli.max_dimension,
        },
    };

    let options = Options {
        cap_bytes: cli.cap,
        resize,
        output_format: cli.format.into(),
        output_dir: cli.out,
        suffix: cli.suffix,
        collision: cli.collision.into(),
        skip_if_under_cap: !cli.force_recompress,
        jpeg_quality_min: cli.quality_min,
        jpeg_quality_max: cli.quality_max,
        metadata: cli.metadata.into(),
        convert_srgb: cli.srgb,
        perceptual_floor,
        rename_pattern: cli.rename.filter(|p| !p.is_empty()),
        ..Options::default()
    };

    let files = scan_inputs(&cli.inputs);
    if files.is_empty() {
        eprintln!("no supported images found in the given paths");
        return ExitCode::FAILURE;
    }

    let quiet = cli.quiet;
    if !quiet {
        println!(
            "Compressing {} image(s) → cap {}",
            files.len(),
            human_bytes(options.cap_bytes)
        );
    }

    let items: Vec<BatchItem> = files
        .iter()
        .map(|f| BatchItem::new(f.path.clone()))
        .collect();

    let cancel = AtomicBool::new(false);
    let summary = compress_batch(&items, &options, &cancel, &|p: Progress| {
        if !quiet {
            println!(
                "[{}/{}] {} — {}",
                p.completed,
                p.total,
                file_name(&p.result.input),
                describe(&p.result.outcome)
            );
        }
    });

    let mut compressed = 0u32;
    let mut skipped = 0u32;
    let mut failed = 0u32;
    for result in &summary.results {
        match &result.outcome {
            Outcome::Compressed { .. } => compressed += 1,
            Outcome::SkippedUnderCap { .. } | Outcome::SkippedCollision => skipped += 1,
            Outcome::Unreachable { .. } | Outcome::Failed { .. } | Outcome::Cancelled => {
                failed += 1
            }
        }
    }

    println!("Done: {compressed} compressed, {skipped} skipped, {failed} failed/unreachable");

    if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
