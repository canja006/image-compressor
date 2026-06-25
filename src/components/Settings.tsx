import { useEffect, useState } from 'react'
import { useStore } from '../store/useStore'
import { pickOutputDir, previewRename } from '../lib/tauri'
import type { CollisionPolicy, MetadataMode } from '../lib/types'
import { basename } from '../lib/outcome'
import { hexToRgb, rgbToHex } from '../lib/color'
import { Button, Field, NumberField, Segmented, Toggle } from './ui'
import { cx } from '../lib/cx'
import { IconChevron } from '../lib/icons'

const BACKGROUND_SWATCHES = ['#ffffff', '#000000', '#f7f6f3'] as const

/** Output-naming pattern input with a live preview of the expanded name (B7). The expansion is done
 *  by the engine via a Tauri command, so the GUI and the real run never diverge. */
function RenameField() {
  const pattern = useStore((s) => s.settings.renamePattern)
  const update = useStore((s) => s.updateSettings)
  const firstInput = useStore((s) => s.inputs[0])
  const previewSource = useStore((s) => s.previewSource)
  const running = useStore((s) => s.phase === 'running')
  const [sample, setSample] = useState('')

  useEffect(() => {
    if (!pattern.trim()) {
      setSample('')
      return
    }
    const stem = firstInput ? basename(firstInput.path).replace(/\.[^.]+$/, '') : 'photo'
    const width = previewSource?.width ?? 1920
    const height = previewSource?.height ?? 1080
    const date = new Date().toISOString().slice(0, 10)
    let cancelled = false
    const timer = setTimeout(() => {
      previewRename(pattern, stem, width, height, date)
        .then((name) => !cancelled && setSample(name))
        .catch(() => !cancelled && setSample(''))
    }, 250)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [pattern, firstInput, previewSource])

  return (
    <Field
      label="Output naming"
      hint="Tokens: {name} {seq:000} {date} {w} {h}. Empty uses the filename + suffix above."
    >
      <input
        type="text"
        value={pattern}
        disabled={running}
        placeholder="{name}-{seq:000}"
        onChange={(e) => update({ renamePattern: e.target.value })}
        aria-label="Output naming pattern"
        className="w-full rounded-md border border-line bg-surface px-3 py-2 font-mono text-xs text-ink outline-none transition-colors focus:border-line-strong"
      />
      {sample !== '' && (
        <p className="font-mono text-[11px] text-faint">
          Example: <span className="text-muted">{sample}</span>
        </p>
      )}
    </Field>
  )
}

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

          {settings.outputFormat === 'jpeg' && (
            <Field
              label="JPEG background"
              hint="Transparent areas are filled with this color — JPEG has no transparency. PNG and AVIF keep it."
            >
              <div className="flex items-center gap-2">
                <input
                  type="color"
                  aria-label="JPEG background color"
                  value={rgbToHex(settings.background)}
                  disabled={running}
                  onChange={(e) => update({ background: hexToRgb(e.target.value) })}
                  className="h-9 w-12 cursor-pointer rounded-md border border-line bg-surface p-1"
                />
                <div className="flex items-center gap-1.5">
                  {BACKGROUND_SWATCHES.map((hex) => (
                    <button
                      key={hex}
                      type="button"
                      aria-label={`Use background ${hex}`}
                      onClick={() => update({ background: hexToRgb(hex) })}
                      className="h-7 w-7 rounded-md border border-line transition-transform duration-150 hover:scale-105"
                      style={{ backgroundColor: hex }}
                    />
                  ))}
                </div>
              </div>
            </Field>
          )}

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

          <div className="space-y-5 border-t border-line pt-5">
            <Field
              label="Metadata"
              hint="Orientation is always applied to the pixels. This controls what EXIF/ICC stays embedded in the file."
            >
              <Segmented<MetadataMode>
                stretch
                ariaLabel="Metadata handling"
                value={settings.metadata}
                onChange={(metadata) => update({ metadata })}
                options={[
                  {
                    value: 'stripAll',
                    label: 'Strip all',
                    title: 'Remove all metadata — smallest and most private',
                  },
                  {
                    value: 'keepOrientationIcc',
                    label: 'Keep ICC',
                    title: 'Re-embed the color profile only',
                  },
                  {
                    value: 'keepAll',
                    label: 'Keep all',
                    title: 'Re-embed the color profile (full EXIF re-embedding is coming)',
                  },
                  {
                    value: 'stripGps',
                    label: 'No GPS',
                    title: 'Keep the color profile but strip location data',
                  },
                ]}
              />
            </Field>

            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs font-medium text-ink">Convert to sRGB</p>
                <p className="text-[11px] text-faint">
                  Normalizes wide-gamut photos for consistent color on the web.
                </p>
              </div>
              <Toggle
                ariaLabel="Convert to sRGB"
                checked={settings.convertSrgb}
                onChange={(convertSrgb) => update({ convertSrgb })}
              />
            </div>

            <Field
              label="Perceptual quality floor"
              hint="Won't ship output below this visual similarity (SSIM) — trades resolution to stay sharp instead of dropping quality."
            >
              <div className="flex items-center gap-2">
                <Toggle
                  ariaLabel="Enable the perceptual quality floor"
                  checked={settings.perceptualFloorEnabled}
                  onChange={(perceptualFloorEnabled) => update({ perceptualFloorEnabled })}
                />
                <NumberField
                  className="flex-1"
                  value={settings.perceptualFloorPct}
                  min={50}
                  max={100}
                  step={1}
                  suffix="% SSIM"
                  disabled={running || !settings.perceptualFloorEnabled}
                  ariaLabel="Perceptual floor percentage"
                  onChange={(v) => update({ perceptualFloorPct: Math.round(v) })}
                />
              </div>
            </Field>

            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs font-medium text-ink">Show quality metrics</p>
                <p className="text-[11px] text-faint">SSIM &amp; PSNR next to the size in the preview.</p>
              </div>
              <Toggle
                ariaLabel="Show quality metrics in the preview"
                checked={settings.showMetrics}
                onChange={(showMetrics) => update({ showMetrics })}
              />
            </div>

            <RenameField />
          </div>
        </div>
      )}
    </div>
  )
}
