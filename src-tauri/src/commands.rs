//! Tauri commands — the thin bridge between the React frontend and the `engine` crate.
//! `Options`, `InputFile`, `BatchSummary`, and `Progress` are engine serde types reused verbatim.

use crate::CancelState;
use base64::Engine as _;
use engine::{
    BatchItem, BatchSummary, InputFile, Options, Preview, PreviewSource, Progress, ResizeMode,
};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

/// A cached decoded+downscaled preview source, so changing only the cap/format doesn't re-decode.
pub struct CacheEntry {
    path: String,
    resize: ResizeMode,
    source: PreviewSource,
}

/// Managed state holding the most recent preview source (single entry — the preview shows one
/// image at a time).
#[derive(Default)]
pub struct PreviewCache(pub Arc<Mutex<Option<CacheEntry>>>);

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
    items: Vec<BatchItem>,
    options: Options,
) -> Result<BatchSummary, String> {
    let cancel = state.0.clone();
    cancel.store(false, Ordering::Relaxed); // reset any flag from a previous run

    let progress_app = app.clone();

    tauri::async_runtime::spawn_blocking(move || {
        engine::compress_batch(&items, &options, &cancel, &move |progress: Progress| {
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
pub async fn preview_sample(
    state: State<'_, PreviewCache>,
    path: String,
    options: Options,
) -> Result<PreviewDto, String> {
    let cache = state.0.clone();
    let resize = options.resize;

    let preview = tauri::async_runtime::spawn_blocking(move || {
        let original_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        // Reuse the cached decoded+sized source when only the cap/format changed; re-decode only
        // when the file or the resize mode (longest-edge cap or exact target) changed.
        let source = {
            let mut guard = match cache.lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
            let hit = matches!(&*guard, Some(e) if e.path == path && e.resize == resize);
            if !hit {
                match engine::prepare_source(Path::new(&path), &resize) {
                    Ok(src) => {
                        *guard = Some(CacheEntry {
                            path: path.clone(),
                            resize,
                            source: src,
                        });
                    }
                    Err(e) => return Preview::failed(original_bytes, e.to_string()),
                }
            }
            match guard.as_ref() {
                Some(e) => e.source.clone(),
                None => {
                    return Preview::failed(
                        original_bytes,
                        "preview source unavailable".to_string(),
                    )
                }
            }
        };

        engine::preview_from_source(&source, original_bytes, &options)
    })
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

/// Expand a rename pattern with sample values for the live preview in Settings. The real batch uses
/// the same `engine::expand_name`, so this is a single source of truth (no duplicated JS logic).
#[tauri::command]
pub fn preview_rename(
    pattern: String,
    stem: String,
    width: u32,
    height: u32,
    date: String,
) -> String {
    let ctx = engine::NameContext {
        stem: &stem,
        seq: 1,
        width,
        height,
        date: &date,
    };
    engine::expand_name(&pattern, &ctx)
}

/// Decoded + downscaled sources for the file-list size estimates, keyed by path, so changing only the
/// cap or format re-searches without re-decoding. `gen` lets a newer pass supersede an in-flight one
/// (so rapid setting tweaks don't pile up slow AVIF searches). Pruned to the current file set.
#[derive(Default)]
pub struct EstimateCache(pub Arc<EstimateCacheInner>);

#[derive(Default)]
pub struct EstimateCacheInner {
    sources: Mutex<HashMap<PathBuf, (ResizeMode, Arc<PreviewSource>)>>,
    gen: AtomicU64,
}

/// Event channel the file list subscribes to for per-image size estimates as they complete.
pub const ESTIMATE_EVENT: &str = "estimate-progress";

/// One image's estimate, tagged with the pass `token` so the UI can ignore superseded passes.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct EstimateProgress {
    token: u64,
    path: String,
    estimate: engine::SizeEstimate,
}

/// Fetch the prepared (decoded + downscaled) source for `path`, decoding on a cache miss. Returns
/// `None` if the image can't be read. The decode happens outside the lock so images decode in
/// parallel; only the brief map lookup/insert is serialized.
fn cached_source(
    cache: &EstimateCacheInner,
    path: &Path,
    resize: ResizeMode,
) -> Option<Arc<PreviewSource>> {
    if let Ok(map) = cache.sources.lock() {
        if let Some((cached_resize, source)) = map.get(path) {
            if *cached_resize == resize {
                return Some(source.clone());
            }
        }
    }
    let prepared = engine::prepare_source_with(path, &resize, engine::ESTIMATE_MAX_DIM).ok()?;
    let arc = Arc::new(prepared);
    if let Ok(mut map) = cache.sources.lock() {
        map.insert(path.to_path_buf(), (resize, arc.clone()));
    }
    Some(arc)
}

/// Estimate the compressed size of every queued image in parallel, emitting an `ESTIMATE_EVENT` per
/// image as it finishes (so rows fill in progressively). Sources are cached, so re-running after a
/// cap/format change is fast (no re-decode). `token` identifies the pass: a newer call supersedes an
/// older one, which then stops doing work. Mirrors the per-file cap a real run would use.
#[tauri::command]
pub async fn estimate_batch(
    app: AppHandle,
    state: State<'_, EstimateCache>,
    items: Vec<BatchItem>,
    options: Options,
    token: u64,
) -> Result<(), String> {
    let cache = state.0.clone();
    cache.gen.store(token, Ordering::Relaxed);

    tauri::async_runtime::spawn_blocking(move || {
        let resize = options.resize;
        items.par_iter().for_each(|item| {
            // A newer pass started — abandon this one rather than burn cores on stale work.
            if cache.gen.load(Ordering::Relaxed) != token {
                return;
            }
            let path = item.path.as_path();
            let cap = item.cap_override.unwrap_or(options.cap_bytes);
            let original_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

            let estimate =
                if options.skip_if_under_cap && original_bytes > 0 && original_bytes <= cap {
                    engine::SizeEstimate::compressed(original_bytes, false)
                } else {
                    match cached_source(&cache, path, resize) {
                        Some(source) => engine::estimate_from_source(&source, &options, cap),
                        None => engine::SizeEstimate::failed(),
                    }
                };

            if cache.gen.load(Ordering::Relaxed) == token {
                let _ = app.emit(
                    ESTIMATE_EVENT,
                    EstimateProgress {
                        token,
                        path: path.display().to_string(),
                        estimate,
                    },
                );
            }
        });

        // Drop cached sources for files no longer queued, so memory tracks the current batch.
        let current: HashSet<PathBuf> = items.iter().map(|item| item.path.clone()).collect();
        if let Ok(mut map) = cache.sources.lock() {
            map.retain(|cached_path, _| current.contains(cached_path));
        }
    })
    .await
    .map_err(|e| format!("estimate task failed to join: {e}"))
}

/// Decode an image and return a small thumbnail as a data URL for the file list (null on failure).
#[tauri::command]
pub async fn thumbnail(path: String, max: u32) -> Result<Option<String>, String> {
    let bytes =
        tauri::async_runtime::spawn_blocking(move || engine::thumbnail(Path::new(&path), max))
            .await
            .map_err(|e| format!("thumbnail task failed to join: {e}"))?;
    Ok(bytes.ok().map(|b| {
        let encoded = base64::engine::general_purpose::STANDARD.encode(&b);
        format!("data:image/jpeg;base64,{encoded}")
    }))
}
