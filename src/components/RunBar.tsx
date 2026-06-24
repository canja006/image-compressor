import { useStore } from '../store/useStore'
import { useCompressor } from '../lib/useCompressor'
import { isTauri } from '../lib/tauri'
import { Button } from './ui'

export function RunBar() {
  const { start, cancel } = useCompressor()
  const count = useStore((s) => s.inputs.length)
  const phase = useStore((s) => s.phase)
  const completed = useStore((s) => s.completed)
  const total = useStore((s) => s.total)
  const settings = useStore((s) => s.settings)
  const resetRun = useStore((s) => s.resetRun)

  const formatLabel = settings.outputFormat === 'keep' ? 'Keep' : settings.outputFormat.toUpperCase()
  const plan = `${formatLabel} · ≤ ${settings.capValue} ${settings.capUnit}`

  if (phase === 'running') {
    const pct = total > 0 ? Math.round((completed / total) * 100) : 0
    return (
      <div className="space-y-3">
        <div className="flex items-baseline justify-between text-xs">
          <span className="font-medium text-ink">Compressing…</span>
          <span className="tabular-nums text-muted">
            {completed} / {total}
          </span>
        </div>
        <div className="h-1.5 w-full overflow-hidden rounded-full bg-sunken">
          <div
            className="h-full rounded-full bg-accent transition-[width] duration-300 ease-out"
            style={{ width: `${pct}%` }}
          />
        </div>
        <Button variant="secondary" className="w-full" onClick={cancel}>
          Cancel
        </Button>
      </div>
    )
  }

  if (phase === 'done') {
    return (
      <Button variant="primary" className="w-full" onClick={resetRun}>
        Compress again
      </Button>
    )
  }

  const disabled = count === 0 || !isTauri()
  return (
    <div className="space-y-2">
      <Button variant="primary" className="w-full" disabled={disabled} onClick={() => void start()}>
        {count === 0 ? 'Add images to start' : `Compress ${count} ${count === 1 ? 'image' : 'images'}`}
      </Button>
      <p className="text-center font-mono text-[11px] text-faint">
        {isTauri() ? plan : 'Launch the desktop app to compress'}
      </p>
    </div>
  )
}
