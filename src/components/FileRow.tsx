import type { InputFile, FileResult } from '../lib/types'
import type { Phase } from '../store/useStore'
import { basename, describeOutcome, finalBytesOf } from '../lib/outcome'
import { formatBytes } from '../lib/format'
import { Badge, Button } from './ui'
import { IconArrowRight, IconClose, IconImage } from '../lib/icons'

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

  return (
    <li
      className="group flex animate-fade-up items-center gap-3 px-4 py-2.5"
      style={{ animationDelay: `${Math.min(index, 12) * 28}ms` }}
    >
      <span className="grid h-9 w-9 shrink-0 place-items-center rounded-md border border-line bg-sunken text-faint">
        <IconImage size={17} />
      </span>

      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-ink">{basename(input.path)}</p>
        <p className="truncate text-[11px] text-faint">
          {view ? view.detail : input.path}
        </p>
      </div>

      <div className="flex items-center gap-2.5 tabular-nums">
        <span className="text-xs text-muted">{formatBytes(input.bytes)}</span>
        {finalBytes != null && (
          <>
            <IconArrowRight size={13} className="text-faint" />
            <span className="text-xs font-medium text-ink">{formatBytes(finalBytes)}</span>
          </>
        )}
      </div>

      <div className="flex w-[104px] shrink-0 items-center justify-end">
        {view ? (
          <Badge tone={view.tone}>{view.label}</Badge>
        ) : pending ? (
          <span className="flex items-center gap-1.5 text-[11px] text-faint">
            <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-faint" />
            waiting
          </span>
        ) : (
          <Button
            variant="ghost"
            onClick={onRemove}
            aria-label={`Remove ${basename(input.path)}`}
            className="opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
          >
            <IconClose size={15} />
          </Button>
        )}
      </div>
    </li>
  )
}
