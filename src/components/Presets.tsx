import { useState } from 'react'
import { useStore } from '../store/useStore'
import {
  DELIVERY_PRESETS,
  describeRecipe,
  loadUserPresets,
  newPresetId,
  recipeOf,
  saveUserPresets,
  type Preset,
} from '../lib/presets'
import { Button } from './ui'
import { cx } from '../lib/cx'

/** B2/B4: full-recipe preset picker on the main screen. Ships delivery presets and lets the user
 *  save the current settings as a named recipe (persisted to localStorage). */
export function Presets() {
  const settings = useStore((s) => s.settings)
  const update = useStore((s) => s.updateSettings)
  const running = useStore((s) => s.phase === 'running')
  const [user, setUser] = useState<Preset[]>(() => loadUserPresets())
  const [saving, setSaving] = useState(false)
  const [name, setName] = useState('')

  const apply = (p: Preset) => update(p.patch)

  const saveCurrent = () => {
    const trimmed = name.trim()
    if (!trimmed) return
    const preset: Preset = {
      id: newPresetId(),
      name: trimmed,
      note: 'Saved recipe',
      patch: recipeOf(settings),
    }
    // Replace any existing preset with the same name so re-saving updates in place.
    const next = [...user.filter((u) => u.name !== trimmed), preset]
    setUser(next)
    saveUserPresets(next)
    setName('')
    setSaving(false)
  }

  const remove = (id: string) => {
    const next = user.filter((u) => u.id !== id)
    setUser(next)
    saveUserPresets(next)
  }

  return (
    <div className="space-y-2.5">
      <div className="flex items-center justify-between">
        <p className="text-xs font-semibold text-ink">Presets</p>
        <button
          type="button"
          disabled={running}
          onClick={() => setSaving((v) => !v)}
          className="text-[11px] font-medium text-muted transition-colors hover:text-ink disabled:opacity-40"
        >
          {saving ? 'Cancel' : 'Save current…'}
        </button>
      </div>

      {saving && (
        <div className="flex items-center gap-1.5">
          <input
            type="text"
            autoFocus
            value={name}
            placeholder="Preset name"
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') saveCurrent()
            }}
            aria-label="New preset name"
            className="min-w-0 flex-1 rounded-md border border-line bg-surface px-3 py-1.5 text-xs text-ink outline-none transition-colors focus:border-line-strong"
          />
          <Button variant="primary" onClick={saveCurrent} disabled={!name.trim()}>
            Save
          </Button>
        </div>
      )}

      <div className="grid grid-cols-1 gap-1.5">
        {DELIVERY_PRESETS.map((p) => (
          <PresetCard key={p.id} preset={p} disabled={running} onApply={() => apply(p)} />
        ))}
        {user.map((p) => (
          <PresetCard
            key={p.id}
            preset={p}
            disabled={running}
            onApply={() => apply(p)}
            onRemove={() => remove(p.id)}
          />
        ))}
      </div>
    </div>
  )
}

function PresetCard({
  preset,
  disabled,
  onApply,
  onRemove,
}: {
  preset: Preset
  disabled: boolean
  onApply: () => void
  onRemove?: () => void
}) {
  return (
    <div
      className={cx(
        'group flex items-center justify-between rounded-lg border border-line px-3 py-2',
        'transition-colors duration-150 hover:border-line-strong hover:bg-sunken/60',
      )}
    >
      <button
        type="button"
        disabled={disabled}
        onClick={onApply}
        className="flex min-w-0 flex-1 flex-col items-start text-left disabled:opacity-50"
      >
        <span className="truncate text-xs font-semibold text-ink">{preset.name}</span>
        <span className="truncate font-mono text-[10.5px] text-muted">
          {preset.builtin ? preset.note : describeRecipe(preset.patch)}
        </span>
      </button>
      {onRemove && (
        <button
          type="button"
          aria-label={`Delete preset ${preset.name}`}
          onClick={onRemove}
          className="ml-2 shrink-0 rounded px-1.5 text-base leading-none text-faint opacity-0 transition-opacity duration-150 hover:text-bad group-hover:opacity-100"
        >
          ×
        </button>
      )}
    </div>
  )
}
