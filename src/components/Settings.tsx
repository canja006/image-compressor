import { useState } from 'react'
import { useStore } from '../store/useStore'
import { pickOutputDir } from '../lib/tauri'
import type { CollisionPolicy } from '../lib/types'
import { basename } from '../lib/outcome'
import { Button, Field, NumberField, Segmented, Toggle } from './ui'
import { cx } from '../lib/cx'
import { IconChevron } from '../lib/icons'

export function Settings() {
  const [open, setOpen] = useState(false)
  const settings = useStore((s) => s.settings)
  const update = useStore((s) => s.updateSettings)
  const running = useStore((s) => s.phase === 'running')

  return (
    <div className="rounded-xl border border-line bg-surface">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
        className="flex w-full items-center justify-between px-4 py-3 text-left"
      >
        <span className="text-sm font-semibold text-ink">Output &amp; advanced</span>
        <IconChevron
          size={16}
          className={cx('text-faint transition-transform duration-200', open && 'rotate-180')}
        />
      </button>

      {open && (
        <div className="animate-fade-in space-y-5 border-t border-line px-4 py-4">
          <Field
            label="Output folder"
            hint="Where compressed copies are written."
            action={
              settings.outputDir != null ? (
                <button
                  type="button"
                  className="text-[11px] font-medium text-muted hover:text-ink"
                  onClick={() => update({ outputDir: null })}
                >
                  Reset
                </button>
              ) : undefined
            }
          >
            <div className="flex items-center gap-2">
              <div className="flex-1 truncate rounded-md border border-line bg-sunken px-3 py-2 text-xs text-muted">
                {settings.outputDir != null ? basename(settings.outputDir) : 'Next to each image'}
              </div>
              <Button
                variant="secondary"
                disabled={running}
                onClick={async () => {
                  const dir = await pickOutputDir()
                  if (dir) update({ outputDir: dir })
                }}
              >
                Change
              </Button>
            </div>
          </Field>

          <Field label="Filename suffix" hint="Appended to each output name.">
            <input
              type="text"
              value={settings.suffix}
              disabled={running}
              onChange={(e) => update({ suffix: e.target.value })}
              aria-label="Filename suffix"
              className="w-full rounded-md border border-line bg-surface px-3 py-2 font-mono text-xs text-ink outline-none transition-colors focus:border-line-strong"
            />
          </Field>

          <Field label="If the output already exists">
            <Segmented<CollisionPolicy>
              stretch
              ariaLabel="Collision policy"
              value={settings.collision}
              onChange={(collision) => update({ collision })}
              options={[
                { value: 'suffix', label: 'Number it' },
                { value: 'overwrite', label: 'Overwrite' },
                { value: 'skip', label: 'Skip' },
              ]}
            />
          </Field>

          <div className="flex items-center justify-between">
            <div>
              <p className="text-xs font-medium text-ink">Skip files already under the cap</p>
              <p className="text-[11px] text-faint">Copies them as-is instead of re-encoding.</p>
            </div>
            <Toggle
              ariaLabel="Skip files already under the cap"
              checked={settings.skipIfUnderCap}
              onChange={(skipIfUnderCap) => update({ skipIfUnderCap })}
            />
          </div>

          <Field label="JPEG quality range" hint="The size search stays within these bounds (1–100).">
            <div className="flex items-center gap-2">
              <NumberField
                className="flex-1"
                value={settings.jpegQualityMin}
                min={1}
                max={100}
                disabled={running}
                ariaLabel="Minimum JPEG quality"
                onChange={(v) => update({ jpegQualityMin: Math.round(v) })}
              />
              <span className="text-xs text-faint">to</span>
              <NumberField
                className="flex-1"
                value={settings.jpegQualityMax}
                min={1}
                max={100}
                disabled={running}
                ariaLabel="Maximum JPEG quality"
                onChange={(v) => update({ jpegQualityMax: Math.round(v) })}
              />
            </div>
          </Field>
        </div>
      )}
    </div>
  )
}
