//! Folder watcher (B1) — the OS-level bridge. The pure ingest decisions (`should_ingest`,
//! `SettleTracker`) live in the engine crate so they are unit-tested without the filesystem; this
//! module owns the `notify` watcher, a single settle/poll worker thread, and the Tauri commands and
//! status events. Exactly one watch runs at a time.
//!
//! Self-reingest is prevented three ways: the watch is non-recursive, an output folder is required
//! and must differ from the watched folder, and `engine::should_ingest` excludes any path that
//! resolves inside the output folder. A vanished folder is reported but never panics the app.

use engine::{compress_batch, should_ingest, BatchItem, Options, Outcome, SettleTracker};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};

/// Event channel the watch panel subscribes to for status updates.
pub const WATCH_EVENT: &str = "watch-event";

/// How often a pending file's size is re-sampled to detect that a copy has finished.
const POLL_INTERVAL: Duration = Duration::from_millis(400);
/// Consecutive equal, non-zero size samples that mark a file as "settled" (done being written).
const REQUIRED_STABLE: u32 = 2;

/// A status update pushed to the frontend. Tagged union so the UI can switch on `kind`.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum WatchEvent {
    /// The watcher is now live on `dir`.
    Started { dir: String },
    /// A settled file is about to be compressed.
    Processing { path: String },
    /// A file finished. `ok` is true for a written/skipped result, false for failure/unreachable.
    Processed {
        path: String,
        ok: bool,
        detail: String,
        output: Option<String>,
    },
    /// A non-fatal watcher error (e.g. the folder was removed). The thread stays alive.
    Error { message: String },
    /// The watcher stopped (user request or disconnect).
    Stopped,
}

/// Owns the worker thread for the active watch. Dropping/stopping signals the thread and joins it;
/// the `notify` watcher itself lives inside the thread and is dropped when the thread exits.
struct WatchHandle {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

/// Managed Tauri state: at most one active watch.
#[derive(Default)]
pub struct WatchState(Mutex<Option<WatchHandle>>);

/// True when two directories resolve to the same location (canonical when possible, else lexical).
fn same_dir(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

/// Signal the worker to stop and wait for it to finish (drops the OS watcher cleanly).
fn stop_handle(mut handle: WatchHandle) {
    handle.stop.store(true, Ordering::Relaxed);
    if let Some(worker) = handle.worker.take() {
        let _ = worker.join();
    }
}

/// Start watching `dir`, compressing each newly dropped image with `options`. Any previous watch is
/// stopped first. Requires an output folder distinct from the watched folder so results are never
/// re-ingested.
#[tauri::command]
pub fn start_watch(
    app: AppHandle,
    state: State<'_, WatchState>,
    dir: String,
    options: Options,
) -> Result<(), String> {
    let watch_dir = PathBuf::from(&dir);
    if !watch_dir.is_dir() {
        return Err(format!("Not a folder: {dir}"));
    }
    let output_dir = options
        .output_dir
        .clone()
        .ok_or("Watch mode needs an output folder so results aren't re-ingested")?;
    if same_dir(&output_dir, &watch_dir) {
        return Err("The output folder must differ from the watched folder".to_string());
    }

    let mut guard = state.0.lock().map_err(|_| "watch state poisoned")?;
    if let Some(existing) = guard.take() {
        stop_handle(existing);
    }

    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let worker_app = app.clone();

    let worker = std::thread::Builder::new()
        .name("watch-folder".to_string())
        .spawn(move || watch_loop(worker_app, watch_dir, options, output_dir, worker_stop))
        .map_err(|e| format!("could not start watcher thread: {e}"))?;

    *guard = Some(WatchHandle {
        stop,
        worker: Some(worker),
    });
    let _ = app.emit(WATCH_EVENT, WatchEvent::Started { dir });
    Ok(())
}

/// Stop the active watch (no-op if none). Always emits `Stopped`.
#[tauri::command]
pub fn stop_watch(app: AppHandle, state: State<'_, WatchState>) -> Result<(), String> {
    let handle = {
        let mut guard = state.0.lock().map_err(|_| "watch state poisoned")?;
        guard.take()
    };
    if let Some(handle) = handle {
        stop_handle(handle);
    }
    let _ = app.emit(WATCH_EVENT, WatchEvent::Stopped);
    Ok(())
}

/// Report whether a watch is currently active (so the UI can restore its toggle on reload).
#[tauri::command]
pub fn watch_status(state: State<'_, WatchState>) -> bool {
    state.0.lock().map(|g| g.is_some()).unwrap_or(false)
}

/// The worker thread: set up the OS watcher, then loop receiving events and polling pending files on
/// a fixed cadence until asked to stop. Owns the `notify` watcher for its whole lifetime.
fn watch_loop(
    app: AppHandle,
    dir: PathBuf,
    options: Options,
    output_dir: PathBuf,
    stop: Arc<AtomicBool>,
) {
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = match RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        Config::default(),
    ) {
        Ok(w) => w,
        Err(e) => {
            let _ = app.emit(
                WATCH_EVENT,
                WatchEvent::Error {
                    message: format!("watcher init failed: {e}"),
                },
            );
            return;
        }
    };
    if let Err(e) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
        let _ = app.emit(
            WATCH_EVENT,
            WatchEvent::Error {
                message: format!("cannot watch {}: {e}", dir.display()),
            },
        );
        return;
    }

    // Per-path size trackers. A file is processed once its size is stable; it's dropped if it
    // vanishes before settling. The cancel flag stays false — single-file ingests aren't cancelled.
    let mut pending: HashMap<PathBuf, SettleTracker> = HashMap::new();
    let cancel = AtomicBool::new(false);
    let mut next_poll = Instant::now() + POLL_INTERVAL;

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let timeout = next_poll.saturating_duration_since(Instant::now());
        match rx.recv_timeout(timeout) {
            Ok(Ok(event)) => {
                if is_ingest_event(&event.kind) {
                    for path in event.paths {
                        if should_ingest(&path, Some(output_dir.as_path())) {
                            pending
                                .entry(path)
                                .or_insert_with(|| SettleTracker::new(REQUIRED_STABLE));
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                // A watch error (e.g. the folder was removed). Report but keep the thread alive so
                // stop stays responsive and a transient error never crashes the app.
                let _ = app.emit(
                    WATCH_EVENT,
                    WatchEvent::Error {
                        message: e.to_string(),
                    },
                );
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        // Sample sizes at most once per interval so a still-writing file isn't mistaken for settled
        // just because two events arrived back to back.
        if Instant::now() >= next_poll {
            drain_settled(&app, &mut pending, &options, &cancel);
            next_poll = Instant::now() + POLL_INTERVAL;
        }
    }
}

/// True for events that mean a file may have appeared or grown (create / write / moved-in).
fn is_ingest_event(kind: &EventKind) -> bool {
    matches!(kind, EventKind::Create(_) | EventKind::Modify(_))
}

/// Re-sample every pending file's size; process the ones that have settled and forget the ones that
/// disappeared.
fn drain_settled(
    app: &AppHandle,
    pending: &mut HashMap<PathBuf, SettleTracker>,
    options: &Options,
    cancel: &AtomicBool,
) {
    let mut ready: Vec<PathBuf> = Vec::new();
    pending.retain(|path, tracker| match std::fs::metadata(path) {
        Ok(meta) => {
            if tracker.observe(meta.len()) {
                ready.push(path.clone());
                false // settled — stop tracking; processed below
            } else {
                true // keep polling
            }
        }
        Err(_) => false, // vanished — drop it
    });
    for path in ready {
        process_file(app, &path, options, cancel);
    }
}

/// Compress a single settled file through the engine and emit its result.
fn process_file(app: &AppHandle, path: &Path, options: &Options, cancel: &AtomicBool) {
    let _ = app.emit(
        WATCH_EVENT,
        WatchEvent::Processing {
            path: path.display().to_string(),
        },
    );
    cancel.store(false, Ordering::Relaxed);
    let items = [BatchItem::new(path.to_path_buf())];
    let summary = compress_batch(&items, options, cancel, &|_p| {});
    if let Some(result) = summary.results.first() {
        let (ok, detail) = summarize(&result.outcome);
        let _ = app.emit(
            WATCH_EVENT,
            WatchEvent::Processed {
                path: path.display().to_string(),
                ok,
                detail,
                output: result.output.as_ref().map(|p| p.display().to_string()),
            },
        );
    }
}

/// Map an engine outcome to a (success, human-readable) pair for the status list.
fn summarize(outcome: &Outcome) -> (bool, String) {
    match outcome {
        Outcome::Compressed {
            final_bytes,
            quality,
            width,
            height,
            downscaled,
        } => {
            let q = quality.map(|q| format!(" q{q}")).unwrap_or_default();
            let scaled = if *downscaled { " ↓" } else { "" };
            (
                true,
                format!(
                    "{} → {width}×{height}{q}{scaled}",
                    human_bytes(*final_bytes)
                ),
            )
        }
        Outcome::SkippedUnderCap { bytes } => {
            (true, format!("already under cap ({})", human_bytes(*bytes)))
        }
        Outcome::SkippedCollision => (true, "skipped (output exists)".to_string()),
        Outcome::Unreachable { reason } => (false, format!("unreachable: {reason}")),
        Outcome::Failed { reason } => (false, format!("failed: {reason}")),
        Outcome::Cancelled => (false, "cancelled".to_string()),
    }
}

/// Compact human-readable byte size for status text.
fn human_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.0} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{bytes} B")
    }
}
