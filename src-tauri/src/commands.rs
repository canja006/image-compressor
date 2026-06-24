//! Tauri commands — the thin bridge between the React frontend and the `engine` crate.
//! `Options`, `InputFile`, `BatchSummary`, and `Progress` are engine serde types reused verbatim.

use crate::CancelState;
use engine::{BatchSummary, InputFile, Options, Progress};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};

/// Event channel the frontend subscribes to for per-file progress.
pub const PROGRESS_EVENT: &str = "compress-progress";

/// Expand the user's selected files/folders into the concrete list of supported images + sizes.
#[tauri::command]
pub fn scan_inputs(paths: Vec<String>) -> Vec<InputFile> {
    let bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    engine::scan_inputs(&bufs)
}

/// Request cancellation of the running batch. Takes effect before the next file starts.
#[tauri::command]
pub fn cancel_batch(state: State<'_, CancelState>) {
    state.0.store(true, Ordering::Relaxed);
}

/// Compress a batch to the target size. Runs on a blocking worker so the UI thread stays
/// responsive; emits a `PROGRESS_EVENT` per file as it finishes and returns the full summary.
#[tauri::command]
pub async fn compress_batch(
    app: AppHandle,
    state: State<'_, CancelState>,
    files: Vec<String>,
    options: Options,
) -> Result<BatchSummary, String> {
    let cancel = state.0.clone();
    cancel.store(false, Ordering::Relaxed); // reset any flag from a previous run

    let bufs: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
    let progress_app = app.clone();

    tauri::async_runtime::spawn_blocking(move || {
        engine::compress_batch(&bufs, &options, &cancel, &move |progress: Progress| {
            // Emitting only fails if the window is gone; nothing to do in that case.
            let _ = progress_app.emit(PROGRESS_EVENT, progress);
        })
    })
    .await
    .map_err(|e| format!("batch task failed to join: {e}"))
}
