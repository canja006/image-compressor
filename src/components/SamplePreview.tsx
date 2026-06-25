import { useEffect, useState } from 'react'
import { useStore, buildOptions } from '../store/useStore'
import { isTauri, previewSample } from '../lib/tauri'
import { formatBytes, parseSizeToBytes, percentSaved } from '../lib/format'
import { splitBudget } from '../lib/budget'
import { basename } from '../lib/outcome'
import { cx } from '../lib/cx'
import type { Preview } from '../lib/types'
import { Badge } from './ui'
import { IconAlert, IconSpinner } from '../lib/icons'

/** Live before/after preview of the first queued image, recomputed (debounced) as settings change. */
export function SamplePreview() {
  const inputs = useStore((s) => s.inputs)
  const selectedPath = useStore((s) => s.selectedPath)
  const settings = useStore((s) => s.settings)
  const phase = useStore((s) => s.phase)
  const setPreviewSource = useStore((s) => s.setPreviewSource)
  const [preview, setPreview] = useState<Preview | null>(null)
  const [loading, setLoading] = useState(false)

  // Preview the clicked image, falling back to the first one.
  const sample = inputs.find((f) => f.path === selectedPath) ?? inputs[0]
  const samplePath = sample?.path

  useEffect(() => {
    // Only preview while configuring (idle) — once compressed, the rows/summary show the results.
    if (!samplePath || !isTauri() || phase !== 'idle') {
      setPreview(null)
      setPreviewSource(null)
      return
    }
    let cancelled = false
    setLoading(true)
    const timer = setTimeout(() => {
      const options = buildOptions(settings)
      // In total-budget mode, preview this image against its split share, not the whole budget.
      if (settings.capMode === 'totalBudget') {
        const share = splitBudget(inputs, parseSizeToBytes(settings.capValue, settings.capUnit))[
          samplePath
        ]
        if (share != null) options.capBytes = share
      }
      previewSample(samplePath, options)
        .then((result) => {
          if (!cancelled) {
            setPreview(result)
            setPreviewSource(
              result && result.sourceWidth > 0
                ? { width: result.sourceWidth, height: result.sourceHeight }
                : null,
            )
            setLoading(false)
          }
        })
        .catch(() => {
          if (!cancelled) {
            setPreview(null)
            setPreviewSource(null)
            setLoading(false)
          }
        })
    }, 350)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [samplePath, settings, phase, inputs, setPreviewSource])

  if (!sample || !isTauri() || phase !== 'idle') return null

  return (
    <section className="shrink-0 rounded-xl border border-line bg-surface p-3">
      <div className="mb-2 flex items-center justify-between gap-2">
        <h2 className="text-xs font-semibold text-ink">Preview</h2>
        <div className="flex min-w-0 items-center gap-1.5">
          {loading && preview && <IconSpinner size={12} className="shrink-0 text-faint" />}
          <span className="truncate text-[11px] text-faint" title={sample.path}>
            {basename(sample.path)}
          </span>
        </div>
      </div>
      <PreviewBody
        preview={preview}
        loading={loading}
        fallbackOriginal={sample.bytes}
        showMetrics={settings.showMetrics}
      />
    </section>
  )
}

function PreviewBody({
  preview,
  loading,
  fallbackOriginal,
  showMetrics,
}: {
  preview: Preview | null
  loading: boolean
  fallbackOriginal: number
  showMetrics: boolean
}) {
  if (!preview) {
    return (
      <div className="flex h-36 flex-col items-center justify-center gap-2.5 text-xs text-faint">
        {loading ? (
          <>
            <IconSpinner size={22} className="text-muted" />
            <span>Computing preview…</span>
          </>
        ) : (
          <span>No preview</span>
        )}
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
          <span className="font-medium text-ink">
            {preview.approx ? '≈ ' : ''}
            {formatBytes(final)}
          </span>
        </span>
        <Badge tone="good">{saved > 0 ? `−${saved}%` : 'no gain'}</Badge>
      </div>
      <p className="font-mono text-[11px] text-faint">
        {preview.width}×{preview.height}
        {preview.quality != null && ` · q${preview.quality}`}
        {preview.downscaled && ' · downscaled'}
        {preview.approx && ' · estimate'}
      </p>
      {showMetrics && (preview.ssim != null || preview.psnr != null) && (
        <p className="font-mono text-[11px] text-faint">
          {preview.ssim != null && `SSIM ${preview.ssim.toFixed(3)}`}
          {preview.ssim != null && preview.psnr != null && ' · '}
          {preview.psnr != null && `PSNR ${preview.psnr.toFixed(1)} dB`}
        </p>
      )}
    </div>
  )
}
