//! Pure-Rust **target-size** image compression engine.
//!
//! This crate has no Tauri or OS-specific code: it decodes an image once, searches in memory
//! for the largest encode that still fits a byte cap, and writes the winner exactly once.
//! The Tauri app depends on this crate; the core algorithm (see [`target::compress_to_target`])
//! is therefore unit-testable without the webview stack.

pub mod batch;
pub mod decode;
pub mod encode;
pub mod error;
pub mod model;
pub mod naming;
pub mod preview;
pub mod resize;
pub mod target;

pub use batch::{compress_batch, is_supported, scan_inputs, SUPPORTED_EXTENSIONS};
pub use encode::EncodeFormat;
pub use error::EngineError;
pub use model::{
    BatchItem, BatchSummary, CollisionPolicy, FileResult, InputFile, Options, Outcome,
    OutputFormat, Progress,
};
pub use preview::{preview, Preview};
pub use target::{compress_to_target, TargetResult};
