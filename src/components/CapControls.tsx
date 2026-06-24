import { useStore } from '../store/useStore'
import type { SizeUnit } from '../lib/format'
import type { OutputFormat } from '../lib/types'
import { Field, NumberField, Segmented, Toggle } from './ui'

const FORMAT_HINT: Record<OutputFormat, string> = {
  jpeg: 'Re-encodes to JPEG. Best compatibility and smallest files.',
  png: 'Lossless PNG. Keeps transparency; size is reached by downscaling only.',
  keep: 'PNG for images with transparency, JPEG otherwise.',
}

export function CapControls() {
  const settings = useStore((s) => s.settings)
  const update = useStore((s) => s.updateSettings)
  const running = useStore((s) => s.phase === 'running')

  return (
    <div className="space-y-5">
      <Field label="Target file size">
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
            { value: 'keep', label: 'Keep original' },
          ]}
        />
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
