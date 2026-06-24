import { useCallback } from 'react'
import { useStore, buildOptions } from '../store/useStore'
import { cancelBatch, compressBatch, onProgress } from './tauri'
import type { BatchItem } from './types'

/** Orchestrates a compression run: subscribes to progress, invokes the batch command, and feeds
 *  results back into the store. Uses `getState()` so the callbacks never go stale. */
export function useCompressor() {
  const start = useCallback(async () => {
    const state = useStore.getState()
    if (state.phase === 'running' || state.inputs.length === 0) return

    const options = buildOptions(state.settings)
    const items: BatchItem[] = state.inputs.map((f) => ({
      path: f.path,
      capOverride: state.capOverrides[f.path] ?? null,
    }))
    state.beginRun()

    const unlisten = await onProgress((p) => useStore.getState().recordProgress(p))
    try {
      const summary = await compressBatch(items, options)
      useStore.getState().endRun(summary)
    } catch (error) {
      useStore.getState().setError(error instanceof Error ? error.message : String(error))
    } finally {
      unlisten()
    }
  }, [])

  const cancel = useCallback(() => {
    void cancelBatch()
  }, [])

  return { start, cancel }
}
