// The single boundary to the Tauri backend. Everything here degrades safely when running
// outside the desktop shell (e.g. `npm run dev` in a browser, or under Vitest): `isTauri()`
// guards the calls so importing this module never throws.

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { open } from '@tauri-apps/plugin-dialog'
import type { BatchItem, BatchSummary, InputFile, Options, Preview, Progress } from './types'

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

/** Subscribe to per-file progress events. Returns an unlisten function. */
export async function onProgress(handler: (p: Progress) => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {}
  return listen<Progress>('compress-progress', (event) => handler(event.payload))
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
