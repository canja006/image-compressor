//! Tauri commands — the thin bridge between the React frontend and the `engine` crate.
//! `Options`, `InputFile`, `BatchSummary`, and `Progress` are engine serde types reused verbatim.

use crate::CancelState;
use base64::Engine as _;
use engine::{BatchSummary, InputFile, Options, Preview, Progress};
use std::path::{Path, PathBuf};
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

/// The engine `Preview` plus a ready-to-display data URL of the compressed result.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewDto {
    #[serde(flatten)]
    meta: Preview,
    data_url: Option<String>,
}

/// Compress a single image in memory (writes nothing) and return its metrics plus a data URL,
/// for the live before/after preview. Runs on a blocking worker.
#[tauri::command]
pub async fn preview_sample(path: String, options: Options) -> Result<PreviewDto, String> {
    let preview =
        tauri::async_runtime::spawn_blocking(move || engine::preview(Path::new(&path), &options))
            .await
            .map_err(|e| format!("preview task failed to join: {e}"))?;

    let data_url = if preview.bytes.is_empty() {
        None
    } else {
        let encoded = base64::engine::general_purpose::STANDARD.encode(&preview.bytes);
        let mime = preview
            .mime
            .clone()
            .unwrap_or_else(|| "image/jpeg".to_string());
        Some(format!("data:{mime};base64,{encoded}"))
    };

    Ok(PreviewDto {
        meta: preview,
        data_url,
    })
}
