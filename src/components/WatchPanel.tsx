import { useEffect, useRef, useState } from 'react'
import { buildOptions, useStore } from '../store/useStore'
import {
  isTauri,
  onWatchEvent,
  pickFolder,
  pickOutputDir,
  startWatch,
  stopWatch,
  watchStatus,
} from '../lib/tauri'
import { basename } from '../lib/outcome'
import { Button, Field } from './ui'
import { cx } from '../lib/cx'
import { IconChevron, IconFolder } from '../lib/icons'

interface LogEntry {
  id: number
  name: string
  ok: boolean
  detail: string
}

const WATCH_PREFS = 'image-compressor.watch'

function loadPrefs(): { watchDir: string | null; outputDir: string | null } {
  try {
    const raw = typeof localStorage !== 'undefined' && localStorage.getItem(WATCH_PREFS)
    if (raw) {
      const p = JSON.parse(raw) as { watchDir?: string | null; outputDir?: string | null }
      return { watchDir: p.watchDir ?? null, outputDir: p.outputDir ?? null }
    }
  } catch {
    // ignore corrupt/private-mode storage
  }
  return { watchDir: null, outputDir: null }
}

function savePrefs(watchDir: string | null, outputDir: string | null): void {
  try {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(WATCH_PREFS, JSON.stringify({ watchDir, outputDir }))
    }
  } catch {
    // best-effort
  }
}

/** A read-only folder chip with a Change action, mirroring the output-folder control in Settings. */
function FolderRow({
  value,
  placeholder,
  disabled,
  onChange,
}: {
  value: string | null
  placeholder: string
  disabled: boolean
  onChange: () => void
}) {
  return (
    <div className="flex items-center gap-2">
      <div className="flex min-w-0 flex-1 items-center gap-2 rounded-md border border-line bg-sunken px-3 py-2">
        <IconFolder size={14} className="shrink-0 text-faint" />
        <span className="truncate text-xs text-muted">
          {value != null ? basename(value) : placeholder}
        </span>
      </div>
      <Button variant="secondary" disabled={disabled} onClick={onChange}>
        Change
      </Button>
    </div>
  )
}

/** Folder watcher (B1). Compresses each image dropped into a watched folder using the current recipe,
 *  writing to a separate output folder so results are never re-ingested. Lives in the advanced area;
 *  the main "drop → preset → go" flow is untouched. */
export function WatchPanel() {
  const [open, setOpen] = useState(false)
  const settings = useStore((s) => s.settings)
  const [dirs, setDirs] = useState(loadPrefs)
  const [active, setActive] = useState(false)
  const [note, setNote] = useState<string | null>(null)
  const [log, setLog] = useState<LogEntry[]>([])
  const nextId = useRef(0)

  const { watchDir, outputDir } = dirs

  // Restore the running state on mount (the watch survives a webview reload) and subscribe to events.
  useEffect(() => {
    let alive = true
    let unlisten: (() => void) | undefined

    watchStatus().then((s) => alive && setActive(s))

    onWatchEvent((e) => {
      switch (e.kind) {
        case 'started':
          setActive(true)
          setNote(`Watching ${basename(e.dir)}`)
          break
        case 'processing':
          setNote(`Compressing ${basename(e.path)}…`)
          break
        case 'processed':
          setLog((l) =>
            [
              { id: nextId.current++, name: basename(e.path), ok: e.ok, detail: e.detail },
              ...l,
            ].slice(0, 8),
          )
          setNote(null)
          break
        case 'error':
          setNote(e.message)
          break
        case 'stopped':
          setActive(false)
          setNote('Stopped')
          break
      }
    }).then((un) => {
      if (alive) unlisten = un
      else un()
    })

    return () => {
      alive = false
      unlisten?.()
    }
  }, [])

  const choose = async (which: 'watch' | 'out') => {
    const dir = which === 'watch' ? await pickFolder() : await pickOutputDir()
    if (!dir) return
    const next =
      which === 'watch' ? { watchDir: dir, outputDir } : { watchDir, outputDir: dir }
    setDirs(next)
    savePrefs(next.watchDir, next.outputDir)
  }

  const sameFolder = watchDir != null && watchDir === outputDir
  const canStart = isTauri() && watchDir != null && outputDir != null && !sameFolder

  const start = async () => {
    if (!watchDir || !outputDir) return
    setNote(null)
    try {
      await startWatch(watchDir, buildOptions({ ...settings, outputDir }))
    } catch (err) {
      setActive(false)
      setNote(String(err))
    }
  }

  const stop = async () => {
    try {
      await stopWatch()
    } catch (err) {
      setNote(String(err))
    }
  }

  return (
    <div className="rounded-xl border border-line bg-surface">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
        className="flex w-full items-center justify-between px-4 py-3 text-left"
      >
        <span className="flex items-center gap-2 text-sm font-semibold text-ink">
          Watch folder
          {active && (
            <span className="flex items-center gap-1.5 rounded-full border border-good/30 bg-good/10 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-good">
              <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-good" />
              Live
            </span>
          )}
        </span>
        <IconChevron
          size={16}
          className={cx('text-faint transition-transform duration-200', open && 'rotate-180')}
        />
      </button>

      {open && (
        <div className="animate-fade-in space-y-4 border-t border-line px-4 py-4">
          <p className="text-[11px] leading-relaxed text-faint">
            Auto-compress every image dropped into a folder using the current preset. Results go to a
            separate output folder so they're never re-compressed.
          </p>

          <Field
            label="Watched folder"
            hint="New images here are compressed automatically."
            action={
              watchDir != null && !active ? (
                <button
                  type="button"
                  className="text-[11px] font-medium text-muted hover:text-ink"
                  onClick={() => {
                    setDirs({ watchDir: null, outputDir })
                    savePrefs(null, outputDir)
                  }}
                >
                  Reset
                </button>
              ) : undefined
            }
          >
            <FolderRow
              value={watchDir}
              placeholder="Choose a folder to watch"
              disabled={active}
              onChange={() => choose('watch')}
            />
          </Field>

          <Field label="Save results to" hint="Must differ from the watched folder.">
            <FolderRow
              value={outputDir}
              placeholder="Choose an output folder"
              disabled={active}
              onChange={() => choose('out')}
            />
          </Field>

          {sameFolder && (
            <p className="text-[11px] font-medium text-warn">
              Pick a different output folder so compressed copies aren't picked up again.
            </p>
          )}

          {active ? (
            <Button variant="secondary" className="w-full justify-center" onClick={stop}>
              Stop watching
            </Button>
          ) : (
            <Button
              variant="primary"
              className="w-full justify-center"
              disabled={!canStart}
              onClick={start}
            >
              Start watching
            </Button>
          )}

          {note != null && (
            <p className="truncate text-[11px] text-muted" title={note}>
              {note}
            </p>
          )}

          {log.length > 0 && (
            <ul className="space-y-1 border-t border-line pt-3">
              {log.map((entry) => (
                <li key={entry.id} className="flex items-center gap-2 text-[11px]">
                  <span
                    className={cx(
                      'h-1.5 w-1.5 shrink-0 rounded-full',
                      entry.ok ? 'bg-good' : 'bg-bad',
                    )}
                  />
                  <span className="truncate text-muted">{entry.name}</span>
                  <span className="ml-auto shrink-0 font-mono text-faint">{entry.detail}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  )
}
