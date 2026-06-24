import { useEffect, useState } from 'react'
import { useStore, buildOptions } from '../store/useStore'
import { isTauri, previewSample } from '../lib/tauri'
import { formatBytes, percentSaved } from '../lib/format'
import { basename } from '../lib/outcome'
import { cx } from '../lib/cx'
import type { Preview } from '../lib/types'
import { Badge } from './ui'
import { IconAlert } from '../lib/icons'

/** Live before/after preview of the first queued image, recomputed (debounced) as settings change. */
export function SamplePreview() {
  const inputs = useStore((s) => s.inputs)
  const settings = useStore((s) => s.settings)
  const phase = useStore((s) => s.phase)
  const [preview, setPreview] = useState<Preview | null>(null)
  const [loading, setLoading] = useState(false)

  const sample = inputs[0]
  const samplePath = sample?.path

  useEffect(() => {
    if (!samplePath || !isTauri() || phase === 'running') {
      setPreview(null)
      return
    }
    let cancelled = false
    setLoading(true)
    const timer = setTimeout(() => {
      previewSample(samplePath, buildOptions(settings))
        .then((result) => {
          if (!cancelled) {
            setPreview(result)
            setLoading(false)
          }
        })
        .catch(() => {
          if (!cancelled) {
            setPreview(null)
            setLoading(false)
          }
        })
    }, 350)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [samplePath, settings, phase])

  if (!sample || !isTauri()) return null

  return (
    <section className="shrink-0 rounded-xl border border-line bg-surface p-3">
      <div className="mb-2 flex items-center justify-between gap-2">
        <h2 className="text-xs font-semibold text-ink">Preview</h2>
        <span className="truncate text-[11px] text-faint" title={sample.path}>
          {basename(sample.path)}
        </span>
      </div>
      <PreviewBody preview={preview} loading={loading} fallbackOriginal={sample.bytes} />
    </section>
  )
}

function PreviewBody({
  preview,
  loading,
  fallbackOriginal,
}: {
  preview: Preview | null
  loading: boolean
  fallbackOriginal: number
}) {
  if (!preview) {
    return (
      <div className="flex h-36 items-center justify-center text-xs text-faint">
        {loading ? 'Computing preview…' : 'No preview'}
      </div>
    )
  }

  if (preview.kind === 'failed') {
    return (
      <div className="flex h-36 flex-col items-center justify-center gap-2 px-4 text-center">
        <IconAlert size={18} className="text-bad" />
        <p className="text-xs text-bad">{preview.error ?? 'Could not read this image'}</p>
      </div>
    )
  }

  if (preview.kind === 'unreachable') {
    return (
      <div className="flex h-36 flex-col items-center justify-center gap-2 px-4 text-center">
        <IconAlert size={18} className="text-warn" />
        <p className="text-xs text-warn">
          The cap can&apos;t be reached for this image, even at the smallest size.
        </p>
      </div>
    )
  }

  const original = preview.originalBytes || fallbackOriginal
  const final = preview.finalBytes ?? 0
  const saved = percentSaved(original, final)

  return (
    <div className={cx('space-y-2.5 transition-opacity duration-150', loading && 'opacity-50')}>
      <div className="bg-checker grid place-items-center overflow-hidden rounded-lg border border-line">
        {preview.dataUrl && (
          <img
            src={preview.dataUrl}
            alt="Compressed preview"
            className="max-h-44 w-auto object-contain"
          />
        )}
      </div>
      <div className="flex items-center justify-between text-xs">
        <span className="tabular-nums text-muted">
          {formatBytes(original)} <span className="text-faint">→</span>{' '}
          <span className="font-medium text-ink">{formatBytes(final)}</span>
        </span>
        <Badge tone="good">{saved > 0 ? `−${saved}%` : 'no gain'}</Badge>
      </div>
      <p className="font-mono text-[11px] text-faint">
        {preview.width}×{preview.height}
        {preview.quality != null && ` · q${preview.quality}`}
        {preview.downscaled && ' · downscaled'}
      </p>
    </div>
  )
}
