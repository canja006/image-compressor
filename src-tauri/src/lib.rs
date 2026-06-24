//! Tauri application shell. The image work lives in the platform-neutral `engine` crate; this
//! layer only exposes commands, manages the cancel flag, and relays progress events to the UI.

mod commands;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Shared cancellation flag for the in-flight batch. Cloned into the engine's worker threads.
#[derive(Default)]
pub struct CancelState(pub Arc<AtomicBool>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(CancelState::default())
        .invoke_handler(tauri::generate_handler![
            commands::scan_inputs,
            commands::compress_batch,
            commands::cancel_batch,
        ])
        .run(tauri::generate_context!());

    // The only place the app may fail fatally: if the webview/runtime can't start there is
    // nothing to recover to, so report and exit rather than `unwrap`/`expect` (see hard rules).
    if let Err(error) = result {
        eprintln!("fatal: Image Compressor failed to start: {error}");
        std::process::exit(1);
    }
}
