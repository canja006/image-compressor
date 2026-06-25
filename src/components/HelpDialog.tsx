import { useEffect } from 'react'
import { Button } from './ui'
import { IconClose } from '../lib/icons'

interface Step {
  title: string
  body: string
}

const STEPS: ReadonlyArray<Step> = [
  {
    title: 'Add images',
    body: 'Drag and drop files or a whole folder onto the window, or use Add files / Add folder. Folders are scanned for JPEG, PNG, WebP, and TIFF images.',
  },
  {
    title: 'Set the target size',
    body: 'Type a cap in KB or MB, or tap a preset — Web (500 KB), Portal (2 MB), Email (10 MB). Switch Per file / Total to cap each image individually or fit the whole set under one combined budget. Under Resize, choose Fit to keep the aspect ratio (optionally capping the longest edge) or Exact to crop-to-fill an exact width × height — the image is scaled and cropped to cover the size with no borders.',
  },
  {
    title: 'Choose a format',
    body: 'JPEG is smallest and most compatible (no transparency). PNG is lossless and keeps transparency. AVIF gives the best ratio but encodes slower. Keep original picks PNG for transparent images, JPEG otherwise.',
  },
  {
    title: 'Compress',
    body: 'Each image is written next to the original (or your chosen output folder) with a -compressed suffix. Click any image in the list to preview its result before you run the batch.',
  },
]

export function HelpDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  useEffect(() => {
    if (!open) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [open, onClose])

  if (!open) return null

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
      aria-label="How to use Image Compressor"
    >
      <button
        type="button"
        aria-label="Close help"
        onClick={onClose}
        className="absolute inset-0 animate-fade-in bg-ink/40 backdrop-blur-sm"
      />
      <div className="relative z-10 max-h-[86vh] w-full max-w-lg animate-fade-up overflow-y-auto rounded-2xl border border-line bg-surface p-6 shadow-panel">
        <div className="mb-5 flex items-start justify-between gap-4">
          <div>
            <h2 className="text-lg font-semibold tracking-tight text-ink">How it works</h2>
            <p className="mt-0.5 text-sm text-muted">Compress images to a target file size — offline.</p>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="grid h-8 w-8 shrink-0 place-items-center rounded-md text-muted transition-colors hover:bg-sunken hover:text-ink"
          >
            <IconClose size={16} />
          </button>
        </div>

        <ol className="space-y-4">
          {STEPS.map((step, i) => (
            <li key={step.title} className="flex gap-3">
              <span className="mt-0.5 grid h-6 w-6 shrink-0 place-items-center rounded-full bg-accent font-mono text-xs font-semibold text-accent-fg">
                {i + 1}
              </span>
              <div>
                <p className="text-sm font-medium text-ink">{step.title}</p>
                <p className="text-[13px] leading-relaxed text-muted">{step.body}</p>
              </div>
            </li>
          ))}
        </ol>

        <div className="mt-5 space-y-3 border-t border-line pt-5">
          <div>
            <p className="text-sm font-medium text-ink">How the target size is met</p>
            <p className="text-[13px] leading-relaxed text-muted">
              Each image is decoded once, then the encoder quality is searched for the largest file
              that still fits your cap. If even the lowest quality is over the cap, the dimensions are
              reduced and the search retries. If it still can&apos;t fit, that file is marked
              unreachable. Files already under the cap are copied as-is.
            </p>
          </div>
          <div>
            <p className="text-sm font-medium text-ink">Good to know</p>
            <p className="text-[13px] leading-relaxed text-muted">
              A corrupt or unreachable file is skipped with a reason — the rest of the batch still
              finishes, and you can cancel mid-run. Nothing is ever uploaded; all processing happens
              on your machine.
            </p>
          </div>
        </div>

        <Button variant="primary" className="mt-6 w-full" onClick={onClose}>
          Got it
        </Button>
      </div>
    </div>
  )
}
