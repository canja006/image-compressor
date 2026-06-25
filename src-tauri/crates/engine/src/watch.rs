//! Pure decision logic for the folder watcher (B1). The OS-level `notify` watcher lives in the
//! Tauri layer; this module only holds the side-effect-free predicates it calls, so they can be
//! unit-tested without touching the filesystem event system.

use std::path::Path;

/// Whether a path the watcher discovered should be processed: it must be a supported image AND must
/// not live inside `output_dir` (so the app never re-ingests its own output and loops forever).
/// When both paths exist, compare canonical forms; if canonicalize fails (e.g. the path was already
/// removed, or output_dir doesn't exist yet), fall back to a lexical `starts_with` check.
pub fn should_ingest(path: &Path, output_dir: Option<&Path>) -> bool {
    if !crate::batch::is_supported(path) {
        return false;
    }

    let Some(output_dir) = output_dir else {
        return true;
    };

    // Prefer canonical comparison so `out/x.jpg` and `../inbox/out/x.jpg` resolve to the same place;
    // fall back to a lexical prefix check when either path can't be canonicalized (not yet on disk).
    let inside = match (
        std::fs::canonicalize(path),
        std::fs::canonicalize(output_dir),
    ) {
        (Ok(canon_path), Ok(canon_out)) => canon_path.starts_with(canon_out),
        _ => path.starts_with(output_dir),
    };
    !inside
}

/// Tracks a file's size across polls to decide when it has "settled" (finished being written) before
/// the watcher processes it. Pure state machine — the caller supplies observed sizes.
pub struct SettleTracker {
    required_stable: u32,
    last_size: Option<u64>,
    streak: u32,
}

impl SettleTracker {
    /// `required_stable` = how many consecutive identical, non-zero observations mean "settled".
    /// A value of 0 is treated as 1 (a single non-zero observation settles immediately).
    pub fn new(required_stable: u32) -> Self {
        Self {
            required_stable: required_stable.max(1),
            last_size: None,
            streak: 0,
        }
    }

    /// Feed the latest observed file size. Returns true once the size has been identical and > 0 for
    /// `required_stable` consecutive observations. A changed size resets the streak; a zero size
    /// (an empty/just-created file) is never considered settled.
    pub fn observe(&mut self, size: u64) -> bool {
        if size == 0 {
            self.streak = 0;
            self.last_size = Some(0);
            return false;
        }

        match self.last_size {
            Some(last) if last == size => self.streak += 1,
            _ => {
                self.last_size = Some(size);
                self.streak = 1;
            }
        }

        self.streak >= self.required_stable
    }
}
