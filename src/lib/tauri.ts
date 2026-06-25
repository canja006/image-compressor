// The single boundary to the Tauri backend. Everything here degrades safely when running
// outside the desktop shell (e.g. `npm run dev` in a browser, or under Vitest): `isTauri()`
// guards the calls so importing this module never throws.

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { open } from '@tauri-apps/plugin-dialog'
import type {
  BatchItem,
  BatchSummary,
  EstimateProgress,
  InputFile,
  Options,
  Preview,
  Progress,
} from './types'

const IMAGE_EXTENSIONS = ['jpg', 'jpeg', 'png', 'webp', 'tif', 'tiff']

/** True when running inside the Tauri webview (vs. a plain browser tab). */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

/** Expand selected files/folders into the concrete list of supported images with sizes. */
export async function scanInputs(paths: string[]): Promise<InputFile[]> {
  if (!isTauri() || paths.length === 0) return []
  return invoke<InputFile[]>('scan_inputs', { paths })
}

/** Run a batch. Progress arrives via `onProgress` events; the resolved value is the summary. */
export async function compressBatch(items: BatchItem[], options: Options): Promise<BatchSummary> {
  return invoke<BatchSummary>('compress_batch', { items, options })
}

/** Ask the backend to cancel the in-flight batch (takes effect before the next file). */
export async function cancelBatch(): Promise<void> {
  if (!isTauri()) return
  await invoke('cancel_batch')
}

/** Compress one image in memory (writes nothing) for the live before/after preview. */
export async function previewSample(path: string, options: Options): Promise<Preview | null> {
  if (!isTauri()) return null
  return invoke<Preview>('preview_sample', { path, options })
}

/** Expand a rename pattern with sample values for the live preview (engine is the source of truth). */
export async function previewRename(
  pattern: string,
  stem: string,
  width: number,
  height: number,
  date: string,
): Promise<string> {
  if (!isTauri()) return pattern
  return invoke<string>('preview_rename', { pattern, stem, width, height, date })
}

/** Estimate the compressed size of many images in parallel (for the file-list readout). Results
 *  arrive via `onEstimateProgress` events tagged with `token`; the promise resolves when the pass is
 *  done. Sources are cached backend-side, so re-running after a cap/format change avoids re-decoding. */
export async function estimateBatch(
  items: BatchItem[],
  options: Options,
  token: number,
): Promise<void> {
  if (!isTauri()) return
  await invoke('estimate_batch', { items, options, token })
}

/** Subscribe to per-image size estimates as they complete. Returns an unlisten function. */
export async function onEstimateProgress(
  handler: (p: EstimateProgress) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {}
  return listen<EstimateProgress>('estimate-progress', (event) => handler(event.payload))
}

/** Small thumbnail (data URL) of an image for the file list, or null if it can't be read. */
export async function getThumbnail(path: string, max: number): Promise<string | null> {
  if (!isTauri()) return null
  return invoke<string | null>('thumbnail', { path, max })
}

/** Subscribe to per-file progress events. Returns an unlisten function. */
export async function onProgress(handler: (p: Progress) => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {}
  return listen<Progress>('compress-progress', (event) => handler(event.payload))
}

/** Status updates from the folder watcher (B1). Mirrors the Rust `WatchEvent` tagged union. */
export type WatchEvent =
  | { kind: 'started'; dir: string }
  | { kind: 'processing'; path: string }
  | { kind: 'processed'; path: string; ok: boolean; detail: string; output: string | null }
  | { kind: 'error'; message: string }
  | { kind: 'stopped' }

/** Start watching `dir`, compressing each dropped image with `options`. The backend rejects when the
 *  output folder is missing or equal to the watched folder (prevents re-ingesting its own output). */
export async function startWatch(dir: string, options: Options): Promise<void> {
  if (!isTauri()) return
  await invoke('start_watch', { dir, options })
}

/** Stop the active folder watch (no-op if none is running). */
export async function stopWatch(): Promise<void> {
  if (!isTauri()) return
  await invoke('stop_watch')
}

/** Whether a folder watch is currently active (so the UI can restore its state after a reload). */
export async function watchStatus(): Promise<boolean> {
  if (!isTauri()) return false
  return invoke<boolean>('watch_status')
}

/** Subscribe to folder-watcher status events. Returns an unlisten function. */
export async function onWatchEvent(handler: (e: WatchEvent) => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {}
  return listen<WatchEvent>('watch-event', (event) => handler(event.payload))
}

type DragState = 'enter' | 'over' | 'drop' | 'leave'

/** Subscribe to native file drag-and-drop over the window. Returns an unlisten function. */
export async function onFileDrop(
  handler: (state: DragState, paths: string[]) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {}
  return getCurrentWebview().onDragDropEvent((event) => {
    const payload = event.payload
    if (payload.type === 'drop') handler('drop', payload.paths)
    else if (payload.type === 'enter') handler('enter', payload.paths)
    else if (payload.type === 'over') handler('over', [])
    else handler('leave', [])
  })
}

/** Native picker for one or more image files. Returns absolute paths. */
export async function pickFiles(): Promise<string[]> {
  if (!isTauri()) return []
  const selection = await open({
    multiple: true,
    directory: false,
    title: 'Select images',
    filters: [{ name: 'Images', extensions: IMAGE_EXTENSIONS }],
  })
  if (selection == null) return []
  return Array.isArray(selection) ? selection : [selection]
}

/** Native picker for a folder of images. Returns its absolute path, or null if cancelled. */
export async function pickFolder(): Promise<string | null> {
  if (!isTauri()) return null
  const selection = await open({ multiple: false, directory: true, title: 'Select a folder' })
  return typeof selection === 'string' ? selection : null
}

/** Native picker for the output directory. Returns its absolute path, or null if cancelled. */
export async function pickOutputDir(): Promise<string | null> {
  if (!isTauri()) return null
  const selection = await open({
    multiple: false,
    directory: true,
    title: 'Choose an output folder',
  })
  return typeof selection === 'string' ? selection : null
}
