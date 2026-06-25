import { useCallback } from 'react'
import { useStore, buildOptions } from '../store/useStore'
import { cancelBatch, compressBatch, onProgress } from './tauri'
import { splitBudget } from './budget'
import { parseSizeToBytes } from './format'
import type { BatchItem } from './types'

/** Orchestrates a compression run: subscribes to progress, invokes the batch command, and feeds
 *  results back into the store. Uses `getState()` so the callbacks never go stale. */
export function useCompressor() {
  const start = useCallback(async () => {
    const state = useStore.getState()
    if (state.phase === 'running' || state.inputs.length === 0) return

    const options = buildOptions(state.settings)
    // In total-budget mode the cap is split across files by size; otherwise use manual overrides.
    const overrides =
      state.settings.capMode === 'totalBudget'
        ? splitBudget(state.inputs, parseSizeToBytes(state.settings.capValue, state.settings.capUnit))
        : state.capOverrides
    const items: BatchItem[] = state.inputs.map((f) => ({
      path: f.path,
      capOverride: overrides[f.path] ?? null,
    }))
    state.beginRun()

    // Set up the listener inside the try so that if even subscribing fails, the catch resets the
    // phase (otherwise the UI would be stuck on "running" forever).
    let unlisten = () => {}
    try {
      unlisten = await onProgress((p) => useStore.getState().recordProgress(p))
      const summary = await compressBatch(items, options)
      useStore.getState().endRun(summary)
    } catch (error) {
      useStore.getState().setError(error instanceof Error ? error.message : String(error))
    } finally {
      unlisten()
    }
  }, [])

  const cancel = useCallback(() => {
    // Flip the UI to "Cancelling…" immediately; the backend honors the flag before the next file,
    // so the run resolves shortly after (the in-flight file still finishes).
    useStore.getState().requestCancel()
    void cancelBatch()
  }, [])

  return { start, cancel }
}
