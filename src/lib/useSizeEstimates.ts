import { useEffect, useRef } from 'react'
import { buildOptions, useStore } from '../store/useStore'
import { estimateBatch, isTauri, onEstimateProgress } from './tauri'
import { splitBudget } from './budget'
import { parseSizeToBytes } from './format'
import type { BatchItem } from './types'

/**
 * Fills each queued image's predicted compressed size into the store so the file list can show a
 * before/after size before any run. Work happens on the backend in parallel (across cores) with the
 * decoded sources cached, so re-estimating after a cap/format tweak is fast; results stream back per
 * image and overwrite in place (no flicker). The per-file cap matches exactly what a real run would
 * use (budget split in total-budget mode, otherwise the override or batch cap).
 *
 * Each pass carries a monotonic `token`; a newer pass supersedes the previous one (the backend stops
 * the stale pass, and any late events for an old token are ignored here).
 */
export function useSizeEstimates() {
  // Subscribe to the inputs that affect estimates so the signature below recomputes on change.
  const inputs = useStore((s) => s.inputs)
  const settings = useStore((s) => s.settings)
  const capOverrides = useStore((s) => s.capOverrides)
  const phase = useStore((s) => s.phase)
  const token = useRef(0)

  // A signature of everything an estimate depends on; when it changes, the effect re-runs.
  const signature = JSON.stringify({
    options: buildOptions(settings),
    capMode: settings.capMode,
    capValue: settings.capValue,
    capUnit: settings.capUnit,
    capOverrides,
    files: inputs.map((f) => `${f.path}:${f.bytes}`),
  })

  useEffect(() => {
    if (!isTauri() || phase !== 'idle') return
    const { inputs, settings, capOverrides, setEstimate } = useStore.getState()
    if (inputs.length === 0) return

    const myToken = ++token.current
    let unlisten = () => {}
    let cancelled = false

    // Small debounce so dragging a slider doesn't fire a pass per frame.
    const timer = setTimeout(async () => {
      if (cancelled) return
      const options = buildOptions(settings)
      const perFileCap =
        settings.capMode === 'totalBudget'
          ? splitBudget(inputs, parseSizeToBytes(settings.capValue, settings.capUnit))
          : capOverrides
      const items: BatchItem[] = inputs.map((f) => ({
        path: f.path,
        capOverride: perFileCap[f.path] ?? null,
      }))

      try {
        unlisten = await onEstimateProgress((p) => {
          if (!cancelled && p.token === myToken) setEstimate(p.path, p.estimate)
        })
        if (cancelled) {
          unlisten()
          return
        }
        await estimateBatch(items, options, myToken)
      } catch {
        // Leave rows with their last estimate; the next pass (or the real run) refreshes them.
      }
    }, 250)

    return () => {
      cancelled = true
      clearTimeout(timer)
      unlisten()
    }
    // `signature` already encodes inputs/settings/capOverrides; `phase` gates to the idle state.
  }, [signature, phase])
}
