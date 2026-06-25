import { useStore, type CapMode } from '../store/useStore'
import type { SizeUnit } from '../lib/format'
import type { OutputFormat, Anchor } from '../lib/types'
import { Field, NumberField, Segmented, Toggle } from './ui'

const FORMAT_HINT: Record<OutputFormat, string> = {
  jpeg: 'Re-encodes to JPEG. Best compatibility and smallest files.',
  png: 'Lossless PNG (optimized with oxipng). Keeps transparency; the cap is reached by downscaling.',
  avif: 'Best compression at a given quality, but noticeably slower to encode.',
  keep: 'PNG for images with transparency, JPEG otherwise.',
}

const AVIF_BATCH_WARN_THRESHOLD = 24

/** Describe what an exact crop-to-fill will do to the previewed image: which edges get trimmed and
 *  whether the target would upscale the source. Falls back to a generic note without source dims. */
function describeExactCrop(
  src: { width: number; height: number } | null,
  tw: number,
  th: number,
): { note: string; upscale: boolean } {
  if (tw <= 0 || th <= 0) return { note: '', upscale: false }
  if (!src || src.width <= 0 || src.height <= 0) {
    return { note: `Scaled and centre-cropped to exactly ${tw}×${th} — no borders added.`, upscale: false }
  }
  const targetAr = tw / th
  const srcAr = src.width / src.height
  if (srcAr > targetAr) {
    const cropW = Math.round(src.height * targetAr)
    const trim = Math.round((src.width - cropW) / 2)
    return {
      note: `Source ${src.width}×${src.height} → ${tw}×${th}, trimming ~${trim} px from each side.`,
      upscale: tw > cropW,
    }
  }
  const cropH = Math.round(src.width / targetAr)
  const trim = Math.round((src.height - cropH) / 2)
  return {
    note: `Source ${src.width}×${src.height} → ${tw}×${th}, trimming ~${trim} px from top and bottom.`,
    upscale: th > cropH,
  }
}

export function CapControls() {
  const settings = useStore((s) => s.settings)
  const update = useStore((s) => s.updateSettings)
  const running = useStore((s) => s.phase === 'running')
  const inputCount = useStore((s) => s.inputs.length)
  const previewSource = useStore((s) => s.previewSource)

  const crop = describeExactCrop(previewSource, settings.exactWidth, settings.exactHeight)

  return (
    <div className="space-y-5">
      <Field
        label={settings.capMode === 'totalBudget' ? 'Total budget' : 'Target file size'}
        hint={
          settings.capMode === 'totalBudget'
            ? 'Split across all images by size so the whole set fits.'
            : undefined
        }
        action={
          <Segmented<CapMode>
            ariaLabel="Cap mode"
            value={settings.capMode}
            onChange={(capMode) => update({ capMode })}
            options={[
              { value: 'perFile', label: 'Per file' },
              { value: 'totalBudget', label: 'Total' },
            ]}
          />
        }
      >
        <div className="flex items-center gap-2">
          <NumberField
            className="flex-1"
            value={settings.capValue}
            min={1}
            step={settings.capUnit === 'MB' ? 0.1 : 1}
            disabled={running}
            ariaLabel="Target size value"
            onChange={(v) => update({ capValue: v })}
          />
          <Segmented<SizeUnit>
            ariaLabel="Size unit"
            value={settings.capUnit}
            onChange={(capUnit) => update({ capUnit })}
            options={[
              { value: 'KB', label: 'KB' },
              { value: 'MB', label: 'MB' },
            ]}
          />
        </div>
      </Field>

      <Field label="Output format" hint={FORMAT_HINT[settings.outputFormat]}>
        <Segmented<OutputFormat>
          stretch
          ariaLabel="Output format"
          value={settings.outputFormat}
          onChange={(outputFormat) => update({ outputFormat })}
          options={[
            { value: 'jpeg', label: 'JPEG' },
            { value: 'png', label: 'PNG' },
            { value: 'avif', label: 'AVIF' },
            { value: 'keep', label: 'Keep' },
          ]}
        />
        {settings.outputFormat === 'avif' && inputCount > AVIF_BATCH_WARN_THRESHOLD && (
          <p className="mt-2 rounded-md bg-warn-bg px-2.5 py-1.5 text-[11px] text-warn">
            AVIF encodes slowly — {inputCount} images may take a while.
          </p>
        )}
      </Field>

      <Field
        label="Resize"
        hint={
          settings.resizeMode === 'fit'
            ? 'Preserves aspect ratio; bounds the longest edge before the size search.'
            : 'Scales and centre-crops to fill an exact size — never adds borders.'
        }
        action={
          <Segmented<'fit' | 'exact'>
            ariaLabel="Resize mode"
            value={settings.resizeMode}
            onChange={(resizeMode) => update({ resizeMode })}
            options={[
              { value: 'fit', label: 'Fit' },
              { value: 'exact', label: 'Exact' },
            ]}
          />
        }
      >
        {settings.resizeMode === 'fit' ? (
          <div className="space-y-2.5">
            <div className="flex items-center justify-between">
              <span className="text-xs text-ink">Limit longest edge</span>
              <Toggle
                ariaLabel="Limit the longest edge"
                checked={settings.maxDimensionEnabled}
                onChange={(maxDimensionEnabled) => update({ maxDimensionEnabled })}
              />
            </div>
            {settings.maxDimensionEnabled && (
              <NumberField
                value={settings.maxDimension}
                min={16}
                step={50}
                suffix="px longest edge"
                disabled={running}
                ariaLabel="Maximum longest edge in pixels"
                onChange={(v) => update({ maxDimension: Math.round(v) })}
              />
            )}
          </div>
        ) : (
          <div className="space-y-2.5">
            <div className="flex items-center gap-2">
              <NumberField
                className="flex-1"
                value={settings.exactWidth}
                min={1}
                step={10}
                suffix="W"
                disabled={running}
                ariaLabel="Exact width in pixels"
                onChange={(v) => update({ exactWidth: Math.round(v) })}
              />
              <span className="text-xs text-faint">×</span>
              <NumberField
                className="flex-1"
                value={settings.exactHeight}
                min={1}
                step={10}
                suffix="H"
                disabled={running}
                ariaLabel="Exact height in pixels"
                onChange={(v) => update({ exactHeight: Math.round(v) })}
              />
            </div>
            <div className="space-y-1.5">
              <span className="block text-xs text-muted">Crop anchor</span>
              <Segmented<Anchor>
                stretch
                ariaLabel="Crop anchor"
                value={settings.exactAnchor}
                onChange={(exactAnchor) => update({ exactAnchor })}
                options={[
                  { value: 'start', label: 'Start' },
                  { value: 'center', label: 'Center' },
                  { value: 'end', label: 'End' },
                ]}
              />
            </div>
            <div className="flex items-center justify-between">
              <span className="text-xs text-ink">Allow upscaling</span>
              <Toggle
                ariaLabel="Allow upscaling when the target is larger than the source"
                checked={settings.exactAllowUpscale}
                onChange={(exactAllowUpscale) => update({ exactAllowUpscale })}
              />
            </div>
            {crop.note && <p className="text-[11px] leading-relaxed text-faint">{crop.note}</p>}
            {crop.upscale && settings.exactAllowUpscale && (
              <p className="rounded-md bg-warn-bg px-2.5 py-1.5 text-[11px] text-warn">
                This image is smaller than the target and will be upscaled.
              </p>
            )}
          </div>
        )}
      </Field>
    </div>
  )
}
