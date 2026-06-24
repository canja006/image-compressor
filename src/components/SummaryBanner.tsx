import { useMemo } from 'react'
import { useStore } from '../store/useStore'
import { formatBytes } from '../lib/format'
import { Badge } from './ui'
import { IconAlert } from '../lib/icons'

interface Tally {
  compressed: number
  savedBytes: number
  underCap: number
  skipped: number
  unreachable: number
  failed: number
  cancelled: number
}

function tally(results: ReturnType<typeof useStore.getState>['results']): Tally {
  const t: Tally = {
    compressed: 0,
    savedBytes: 0,
    underCap: 0,
    skipped: 0,
    unreachable: 0,
    failed: 0,
    cancelled: 0,
  }
  for (const r of Object.values(results)) {
    switch (r.outcome.kind) {
      case 'compressed':
        t.compressed += 1
        t.savedBytes += Math.max(0, r.originalBytes - r.outcome.finalBytes)
        break
      case 'skippedUnderCap':
        t.underCap += 1
        break
      case 'skippedCollision':
        t.skipped += 1
        break
      case 'unreachable':
        t.unreachable += 1
        break
      case 'failed':
        t.failed += 1
        break
      case 'cancelled':
        t.cancelled += 1
        break
    }
  }
  return t
}

export function SummaryBanner() {
  const phase = useStore((s) => s.phase)
  const results = useStore((s) => s.results)
  const error = useStore((s) => s.error)
  const t = useMemo(() => tally(results), [results])

  if (phase !== 'done') return null

  if (error) {
    return (
      <div className="flex animate-fade-up items-start gap-3 rounded-xl border border-bad/30 bg-bad-bg px-4 py-3">
        <IconAlert size={18} className="mt-0.5 shrink-0 text-bad" />
        <div>
          <p className="text-sm font-semibold text-bad">The batch could not run</p>
          <p className="text-xs text-bad/80">{error}</p>
        </div>
      </div>
    )
  }

  const headline =
    t.savedBytes > 0
      ? `Done — saved ${formatBytes(t.savedBytes)}`
      : t.compressed > 0
        ? 'Done'
        : 'Finished with no compression'

  return (
    <div className="animate-fade-up rounded-xl border border-line bg-surface px-4 py-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-sm font-semibold text-ink">{headline}</p>
        <div className="flex flex-wrap items-center gap-1.5">
          {t.compressed > 0 && <Badge tone="good">{t.compressed} compressed</Badge>}
          {t.underCap > 0 && <Badge tone="info">{t.underCap} under cap</Badge>}
          {t.skipped > 0 && <Badge tone="warn">{t.skipped} skipped</Badge>}
          {t.unreachable > 0 && <Badge tone="bad">{t.unreachable} unreachable</Badge>}
          {t.failed > 0 && <Badge tone="bad">{t.failed} failed</Badge>}
          {t.cancelled > 0 && <Badge tone="neutral">{t.cancelled} cancelled</Badge>}
        </div>
      </div>
    </div>
  )
}
