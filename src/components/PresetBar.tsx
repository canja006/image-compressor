import { useStore } from '../store/useStore'
import type { SizeUnit } from '../lib/format'
import { cx } from '../lib/cx'

interface Preset {
  id: string
  label: string
  value: number
  unit: SizeUnit
  note: string
}

// Common real-world caps (spec section 3, tier 2 presets).
const PRESETS: ReadonlyArray<Preset> = [
  { id: 'web', label: 'Web', value: 500, unit: 'KB', note: '500 KB' },
  { id: 'portal', label: 'Portal', value: 2, unit: 'MB', note: '2 MB' },
  { id: 'email', label: 'Email', value: 10, unit: 'MB', note: '10 MB' },
]

export function PresetBar() {
  const capValue = useStore((s) => s.settings.capValue)
  const capUnit = useStore((s) => s.settings.capUnit)
  const update = useStore((s) => s.updateSettings)
  const running = useStore((s) => s.phase === 'running')

  return (
    <div className="grid grid-cols-3 gap-1.5">
      {PRESETS.map((p) => {
        const active = capValue === p.value && capUnit === p.unit
        return (
          <button
            key={p.id}
            type="button"
            disabled={running}
            aria-pressed={active}
            onClick={() => update({ capValue: p.value, capUnit: p.unit })}
            className={cx(
              'flex flex-col items-start rounded-lg border px-3 py-2 text-left transition-colors duration-150 disabled:opacity-50',
              active
                ? 'border-accent/30 bg-sunken'
                : 'border-line hover:border-line-strong hover:bg-sunken/60',
            )}
          >
            <span className="text-xs font-semibold text-ink">{p.label}</span>
            <span className="font-mono text-[11px] text-muted">{p.note}</span>
          </button>
        )
      })}
    </div>
  )
}
