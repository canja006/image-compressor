import { useEffect, useMemo, useState } from 'react'
import { useStore } from '../store/useStore'
import { isTauri, onFileDrop, pickFiles, pickFolder, scanInputs } from '../lib/tauri'
import { formatBytes } from '../lib/format'
import { Button } from './ui'
import { cx } from '../lib/cx'
import { FileRow } from './FileRow'
import { IconFolder, IconTray } from '../lib/icons'

export function Intake() {
  const inputs = useStore((s) => s.inputs)
  const results = useStore((s) => s.results)
  const phase = useStore((s) => s.phase)
  const addInputs = useStore((s) => s.addInputs)
  const removeInput = useStore((s) => s.removeInput)
  const clearInputs = useStore((s) => s.clearInputs)

  const [dragActive, setDragActive] = useState(false)
  const [busy, setBusy] = useState(false)
  const running = phase === 'running'

  const totalBytes = useMemo(() => inputs.reduce((sum, f) => sum + f.bytes, 0), [inputs])

  async function ingest(paths: string[]) {
    if (paths.length === 0) return
    setBusy(true)
    try {
      addInputs(await scanInputs(paths))
    } finally {
      setBusy(false)
    }
  }

  useEffect(() => {
    let unlisten: (() => void) | undefined
    let cancelled = false
    // Use the store's stable getState() and module-level helpers so this effect has no reactive
    // dependencies and genuinely runs once (no exhaustive-deps suppression needed).
    const handleDrop = async (paths: string[]) => {
      if (paths.length === 0) return
      setBusy(true)
      try {
        useStore.getState().addInputs(await scanInputs(paths))
      } finally {
        setBusy(false)
      }
    }
    onFileDrop((state, paths) => {
      if (state === 'enter') setDragActive(true)
      else if (state === 'leave') setDragActive(false)
      else if (state === 'drop') {
        setDragActive(false)
        void handleDrop(paths)
      }
    }).then((fn) => {
      if (cancelled) fn()
      else unlisten = fn
    })
    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [])

  const empty = inputs.length === 0

  return (
    <div className="relative flex h-full flex-col">
      <div className="flex items-center justify-between gap-3 px-1 pb-3">
        <div className="flex items-baseline gap-2">
          <h2 className="text-sm font-semibold text-ink">Images</h2>
          {!empty && (
            <span className="text-xs text-faint tabular-nums">
              {inputs.length} · {formatBytes(totalBytes)}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1.5">
          <Button variant="secondary" disabled={running || busy} onClick={async () => ingest(await pickFiles())}>
            <IconTray size={15} /> Add files
          </Button>
          <Button
            variant="secondary"
            disabled={running || busy}
            onClick={async () => {
              const dir = await pickFolder()
              if (dir) void ingest([dir])
            }}
          >
            <IconFolder size={15} /> Add folder
          </Button>
          {!empty && (
            <Button variant="ghost" disabled={running} onClick={clearInputs}>
              Clear
            </Button>
          )}
        </div>
      </div>

      <div className="relative flex-1 overflow-hidden rounded-xl border border-line bg-surface">
        {empty ? (
          <EmptyState onAddFiles={async () => ingest(await pickFiles())} />
        ) : (
          <ul className="h-full divide-y divide-line overflow-y-auto">
            {inputs.map((f, i) => (
              <FileRow
                key={f.path}
                input={f}
                index={i}
                result={results[f.path]}
                phase={phase}
                onRemove={() => removeInput(f.path)}
              />
            ))}
          </ul>
        )}

        {dragActive && (
          <div className="pointer-events-none absolute inset-1.5 z-10 grid place-items-center rounded-lg border-2 border-dashed border-accent/40 bg-canvas/80 backdrop-blur-sm">
            <p className="text-sm font-medium text-ink">Drop to add images</p>
          </div>
        )}
      </div>
    </div>
  )
}

function EmptyState({ onAddFiles }: { onAddFiles: () => void }) {
  const tauri = isTauri()
  return (
    <button
      type="button"
      onClick={onAddFiles}
      disabled={!tauri}
      className={cx(
        'flex h-full w-full flex-col items-center justify-center gap-4 px-8 text-center',
        'transition-colors duration-200',
        tauri && 'hover:bg-sunken/60',
      )}
    >
      <span className="grid h-16 w-16 place-items-center rounded-2xl border border-line bg-sunken text-faint">
        <IconTray size={28} />
      </span>
      <div className="space-y-1">
        <p className="text-base font-medium text-ink">Drop images or a folder here</p>
        <p className="text-sm text-muted">
          {tauri ? 'or click to choose files — JPEG, PNG, WebP, TIFF' : 'Run the desktop app to add files'}
        </p>
      </div>
    </button>
  )
}
