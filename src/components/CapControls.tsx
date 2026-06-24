import { useStore, type CapMode } from '../store/useStore'
import type { SizeUnit } from '../lib/format'
import type { OutputFormat } from '../lib/types'
import { Field, NumberField, Segmented, Toggle } from './ui'

const FORMAT_HINT: Record<OutputFormat, string> = {
  jpeg: 'Re-encodes to JPEG. Best compatibility and smallest files.',
  png: 'Lossless PNG (optimized with oxipng). Keeps transparency; the cap is reached by downscaling.',
  avif: 'Best compression at a given quality, but noticeably slower to encode.',
  keep: 'PNG for images with transparency, JPEG otherwise.',
}

const AVIF_BATCH_WARN_THRESHOLD = 24

export function CapControls() {
  const settings = useStore((s) => s.settings)
  const update = useStore((s) => s.updateSettings)
  const running = useStore((s) => s.phase === 'running')
  const inputCount = useStore((s) => s.inputs.length)

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
        label="Limit dimensions"
        action={
          <Toggle
            ariaLabel="Limit the longest edge"
            checked={settings.maxDimensionEnabled}
            onChange={(maxDimensionEnabled) => update({ maxDimensionEnabled })}
          />
        }
        hint={settings.maxDimensionEnabled ? 'The longest edge is capped before the size search.' : undefined}
      >
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
      </Field>
    </div>
  )
}
