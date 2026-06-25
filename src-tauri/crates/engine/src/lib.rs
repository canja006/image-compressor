//! Pure-Rust **target-size** image compression engine.
//!
//! This crate has no Tauri or OS-specific code: it decodes an image once, searches in memory
//! for the largest encode that still fits a byte cap, and writes the winner exactly once.
//! The Tauri app depends on this crate; the core algorithm (see [`target::compress_to_target`])
//! is therefore unit-testable without the webview stack.

pub mod batch;
pub mod crop;
pub mod decode;
pub mod encode;
pub mod error;
pub mod metadata;
pub mod metrics;
pub mod model;
pub mod naming;
pub mod preview;
pub mod rename;
pub mod resize;
pub mod target;
pub mod watch;

pub use batch::{compress_batch, is_supported, scan_inputs, SUPPORTED_EXTENSIONS};
pub use crop::{cover_crop_rect, cover_crop_resize, CropRect};
pub use encode::EncodeFormat;
pub use error::EngineError;
pub use metadata::{apply_color_and_orientation, mux_metadata, read_source_meta, SourceMeta};
pub use model::{
    Anchor, BatchItem, BatchSummary, CollisionPolicy, FileResult, InputFile, MetadataMode, Options,
    Outcome, OutputFormat, Progress, ResizeMode,
};
pub use preview::{
    estimate_size, prepare_source, preview, preview_from_source, thumbnail, Preview, PreviewSource,
    SizeEstimate,
};
pub use rename::{expand_name, NameContext};
pub use target::{compress_to_target, TargetResult};
pub use watch::{should_ingest, SettleTracker};
