//! Single-image preview: run the size search in memory and report what the user would get, plus
//! the encoded bytes for a live before/after readout. Nothing is written to disk.

use crate::batch::resolve_format;
use crate::decode::decode;
use crate::model::Options;
use crate::resize::downscale_to_long_edge;
use crate::target::compress_to_target;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub original_bytes: u64,
    pub source_width: u32,
    pub source_height: u32,
    pub has_alpha: bool,
    /// `"compressed"` | `"unreachable"` | `"failed"`.
    pub kind: String,
    pub final_bytes: Option<u64>,
    pub quality: Option<u8>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub downscaled: bool,
    pub mime: Option<String>,
    pub error: Option<String>,
    /// Encoded preview bytes (only when `kind == "compressed"`). Skipped in serialization — the
    /// command base64-encodes these into a data URL for the webview instead of sending raw bytes.
    #[serde(skip)]
    pub bytes: Vec<u8>,
}

impl Preview {
    fn failed(original_bytes: u64, error: String) -> Self {
        Preview {
            original_bytes,
            source_width: 0,
            source_height: 0,
            has_alpha: false,
            kind: "failed".to_string(),
            final_bytes: None,
            quality: None,
            width: None,
            height: None,
            downscaled: false,
            mime: None,
            error: Some(error),
            bytes: Vec::new(),
        }
    }
}

/// Compress one image entirely in memory and return the result plus its encoded bytes.
pub fn preview(path: &Path, options: &Options) -> Preview {
    let original_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let raw = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => return Preview::failed(original_bytes, format!("read: {e}")),
    };
    let img = match decode(&raw) {
        Ok(i) => i,
        Err(e) => return Preview::failed(original_bytes, e.to_string()),
    };

    let fmt = resolve_format(options.output_format, &img, options.background);
    let mut p = Preview {
        original_bytes,
        source_width: img.width(),
        source_height: img.height(),
        has_alpha: img.color().has_alpha(),
        kind: "unreachable".to_string(),
        final_bytes: None,
        quality: None,
        width: None,
        height: None,
        downscaled: false,
        mime: None,
        error: None,
        bytes: Vec::new(),
    };

    let base = match options.max_dimension {
        Some(maxd) => match downscale_to_long_edge(&img, maxd) {
            Ok(i) => i,
            Err(e) => return Preview::failed(original_bytes, e.to_string()),
        },
        None => img,
    };

    match compress_to_target(&base, options.cap_bytes, fmt, options) {
        Ok(Some(t)) => {
            p.kind = "compressed".to_string();
            p.final_bytes = Some(t.bytes.len() as u64);
            p.quality = t.quality;
            p.width = Some(t.width);
            p.height = Some(t.height);
            p.downscaled = t.downscaled;
            p.mime = Some(fmt.mime().to_string());
            p.bytes = t.bytes;
        }
        Ok(None) => {}
        Err(e) => return Preview::failed(original_bytes, e.to_string()),
    }
    p
}
