import { useEffect, useRef } from 'react'
import { buildOptions, useStore } from '../store/useStore'
import { estimateSize, isTauri } from './tauri'
import { splitBudget } from './budget'
import { parseSizeToBytes } from './format'
import type { Options } from './types'

/**
 * Fills each queued image's predicted compressed size into the store, one by one, while idle — so
 * the file list can show a before/after size before any run. The per-file cap matches exactly what a
 * real run would use (budget split in total-budget mode, otherwise the per-file override or the batch
 * cap), so the estimate lines up with the result.
 *
 * Recomputes (debounced) whenever a size-affecting setting, a cap override, or the file set changes;
 * a generation token aborts an in-flight pass cleanly when that happens.
 */
export function useSizeEstimates() {
  // Subscribe to the inputs that affect estimates so the signature below recomputes on change.
  const inputs = useStore((s) => s.inputs)
  const settings = useStore((s) => s.settings)
  const capOverrides = useStore((s) => s.capOverrides)
  const phase = useStore((s) => s.phase)
  const gen = useRef(0)

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
    // Read the current values fresh so the effect's only reactive deps are `signature` + `phase`.
    const { inputs, settings, capOverrides, setEstimate, clearEstimates } = useStore.getState()
    if (inputs.length === 0) return

    const myGen = ++gen.current
    clearEstimates()

    const base = buildOptions(settings)
    const perFileCap =
      settings.capMode === 'totalBudget'
        ? splitBudget(inputs, parseSizeToBytes(settings.capValue, settings.capUnit))
        : capOverrides

    let cancelled = false
    const timer = setTimeout(async () => {
      for (const file of inputs) {
        if (cancelled || gen.current !== myGen) return
        const options: Options = { ...base, capBytes: perFileCap[file.path] ?? base.capBytes }
        try {
          const estimate = await estimateSize(file.path, options)
          if (!cancelled && gen.current === myGen && estimate) setEstimate(file.path, estimate)
        } catch {
          // Leave this row without an estimate; the next pass (or the real run) will fill it.
        }
      }
    }, 350)

    return () => {
      cancelled = true
      clearTimeout(timer)
    }
    // `signature` already encodes inputs/settings/capOverrides; `phase` gates to the idle state.
  }, [signature, phase])
}
