import { useEffect, useState } from 'react'
import type { InputFile, FileResult, SizeEstimate } from '../lib/types'
import { useStore, type Phase } from '../store/useStore'
import { basename, describeOutcome, finalBytesOf } from '../lib/outcome'
import { formatBytes, parseSizeToBytes, type SizeUnit } from '../lib/format'
import { cachedThumbnail, loadThumbnail } from '../lib/thumbnails'
import { cx } from '../lib/cx'
import { Badge, Button, NumberField } from './ui'
import { IconArrowRight, IconCheck, IconClose, IconImage } from '../lib/icons'

interface FileRowProps {
  input: InputFile
  result: FileResult | undefined
  phase: Phase
  index: number
  onRemove: () => void
}

export function FileRow({ input, result, phase, index, onRemove }: FileRowProps) {
  const view = result ? describeOutcome(result) : null
  const finalBytes = result ? finalBytesOf(result) : null
  const pending = phase === 'running' && !result
  const capMode = useStore((s) => s.settings.capMode)
  const estimate = useStore((s) => s.estimates[input.path])
  const selectedPath = useStore((s) => s.selectedPath)
  const selectPreview = useStore((s) => s.selectPreview)
  // The previewed row: the explicit selection, or the first row when nothing is selected.
  const isSelected = selectedPath != null ? selectedPath === input.path : index === 0

  const [thumb, setThumb] = useState<string | null>(() => cachedThumbnail(input.path))
  useEffect(() => {
    let alive = true
    void loadThumbnail(input.path).then((url) => {
      if (alive && url) setThumb(url)
    })
    return () => {
      alive = false
    }
  }, [input.path])

  return (
    <li
      className={cx(
        'group flex animate-fade-up items-center gap-3 px-4 py-2.5 transition-colors',
        isSelected && phase === 'idle' && 'bg-sunken/60',
      )}
      style={{ animationDelay: `${Math.min(index, 12) * 28}ms` }}
    >
      <button
        type="button"
        onClick={() => selectPreview(input.path)}
        title="Show this image in the preview"
        className="flex min-w-0 flex-1 items-center gap-3 text-left"
      >
        {thumb ? (
          <img
            src={thumb}
            alt=""
            className={cx(
              'h-9 w-9 shrink-0 rounded-md border object-cover',
              isSelected ? 'border-accent/40 ring-2 ring-accent/25' : 'border-line',
            )}
          />
        ) : (
          <span
            className={cx(
              'grid h-9 w-9 shrink-0 place-items-center rounded-md border bg-sunken text-faint',
              isSelected ? 'border-accent/40 ring-2 ring-accent/25' : 'border-line',
            )}
          >
            <IconImage size={17} />
          </span>
        )}

        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium text-ink">{basename(input.path)}</p>
          <p className="truncate text-[11px] text-faint">{view ? view.detail : input.path}</p>
        </div>
      </button>

      <div className="flex items-center gap-2.5 tabular-nums">
        <span className="text-xs text-muted">{formatBytes(input.bytes)}</span>
        {finalBytes != null ? (
          <>
            <IconArrowRight size={13} className="text-faint" />
            <span className="text-xs font-medium text-ink">{formatBytes(finalBytes)}</span>
          </>
        ) : (
          !result && phase === 'idle' && <EstimatedSize estimate={estimate} />
        )}
      </div>

      <div className="flex shrink-0 items-center justify-end gap-1.5">
        {view ? (
          <Badge tone={view.tone}>{view.label}</Badge>
        ) : pending ? (
          <span className="flex items-center gap-1.5 text-[11px] text-faint">
            <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-faint" />
            waiting
          </span>
        ) : (
          <>
            {capMode === 'perFile' && <CapOverrideControl path={input.path} />}
            <Button
              variant="ghost"
              onClick={onRemove}
              aria-label={`Remove ${basename(input.path)}`}
              className="opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
            >
              <IconClose size={15} />
            </Button>
          </>
        )}
      </div>
    </li>
  )
}

/** Predicted compressed size shown before a run (filled in lazily by `useSizeEstimates`). A `~`
 *  prefix and muted styling distinguish it from the exact size shown after the run. */
function EstimatedSize({ estimate }: { estimate: SizeEstimate | undefined }) {
  if (estimate == null) return null // still computing — it fills in shortly
  if (estimate.kind === 'compressed' && estimate.finalBytes != null) {
    return (
      <>
        <IconArrowRight size={13} className="text-faint" />
        <span className="animate-fade-in text-xs text-muted" title="Estimated — run for the exact size">
          ~{formatBytes(estimate.finalBytes)}
        </span>
      </>
    )
  }
  if (estimate.kind === 'unreachable') {
    return (
      <span
        className="animate-fade-in text-[11px] font-medium text-warn"
        title="The cap can't be reached for this image, even at the smallest size"
      >
        cap too small
      </span>
    )
  }
  return null // failed — the row's detail line already surfaces read errors
}

function toUnit(bytes: number, unit: SizeUnit): number {
  return unit === 'MB' ? Math.round((bytes / (1024 * 1024)) * 10) / 10 : Math.round(bytes / 1024)
}

/** Per-file cap override: a chip when set, a hover affordance when not, and a compact inline editor. */
function CapOverrideControl({ path }: { path: string }) {
  const unit = useStore((s) => s.settings.capUnit)
  const defaultValue = useStore((s) => s.settings.capValue)
  const override = useStore((s) => s.capOverrides[path])
  const setCapOverride = useStore((s) => s.setCapOverride)

  const [editing, setEditing] = useState(false)
  const [value, setValue] = useState(0)

  function open() {
    setValue(override != null ? toUnit(override, unit) : defaultValue)
    setEditing(true)
  }

  if (editing) {
    return (
      <div className="flex items-center gap-1">
        <NumberField
          className="w-[96px]"
          value={value}
          min={1}
          suffix={unit}
          ariaLabel="Per-file cap"
          onChange={setValue}
        />
        <button
          type="button"
          aria-label="Apply per-file cap"
          onClick={() => {
            setCapOverride(path, parseSizeToBytes(value, unit))
            setEditing(false)
          }}
          className="grid h-7 w-7 place-items-center rounded-md text-good transition-colors hover:bg-good-bg"
        >
          <IconCheck size={14} />
        </button>
        <button
          type="button"
          aria-label="Use the batch cap"
          onClick={() => {
            setCapOverride(path, null)
            setEditing(false)
          }}
          className="grid h-7 w-7 place-items-center rounded-md text-muted transition-colors hover:bg-sunken hover:text-ink"
        >
          <IconClose size={14} />
        </button>
      </div>
    )
  }

  if (override != null) {
    return (
      <button
        type="button"
        onClick={open}
        title="Per-file cap (click to edit)"
        className="rounded-full bg-info-bg px-2 py-0.5 text-[10.5px] font-semibold uppercase tracking-[0.04em] text-info"
      >
        ≤ {formatBytes(override)}
      </button>
    )
  }

  return (
    <button
      type="button"
      onClick={open}
      className="text-[11px] text-faint opacity-0 transition-opacity hover:text-ink group-hover:opacity-100 focus-visible:opacity-100"
    >
      cap
    </button>
  )
}
