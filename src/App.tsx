import { useState } from 'react'
import { Intake } from './components/Intake'
import { PresetBar } from './components/PresetBar'
import { CapControls } from './components/CapControls'
import { Settings } from './components/Settings'
import { RunBar } from './components/RunBar'
import { SummaryBanner } from './components/SummaryBanner'
import { SamplePreview } from './components/SamplePreview'
import { HelpDialog } from './components/HelpDialog'
import { Panel } from './components/ui'
import { IconMoon, IconSun } from './lib/icons'
import { applyTheme, getInitialTheme, type Theme } from './lib/theme'

function Wordmark() {
  return (
    <span className="grid h-8 w-8 place-items-center rounded-lg border border-line bg-surface text-ink shadow-subtle">
      <svg
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        <path d="M3 12h7m0 0L7 9m3 3-3 3" />
        <path d="M21 12h-7m0 0 3-3m-3 3 3 3" />
      </svg>
    </span>
  )
}

function ThemeToggle({ theme, onToggle }: { theme: Theme; onToggle: () => void }) {
  return (
    <button
      type="button"
      onClick={onToggle}
      aria-label={`Switch to ${theme === 'dark' ? 'light' : 'dark'} theme`}
      className="grid h-8 w-8 place-items-center rounded-md border border-line bg-surface text-muted transition-[color,border-color,transform] duration-150 hover:border-line-strong hover:text-ink active:scale-95"
    >
      {theme === 'dark' ? <IconSun size={16} /> : <IconMoon size={16} />}
    </button>
  )
}

function AmbientBackground() {
  return (
    <div
      aria-hidden="true"
      className="pointer-events-none fixed inset-0 z-0 animate-drift"
      style={{
        background:
          'radial-gradient(55% 45% at 12% -5%, rgb(var(--c-ink) / 0.045), transparent 70%),' +
          'radial-gradient(45% 40% at 105% 105%, rgb(var(--c-ink) / 0.035), transparent 70%)',
      }}
    />
  )
}

export default function App() {
  const [theme, setTheme] = useState<Theme>(() => getInitialTheme())
  const [helpOpen, setHelpOpen] = useState(false)

  function toggleTheme() {
    const next: Theme = theme === 'dark' ? 'light' : 'dark'
    setTheme(next)
    applyTheme(next)
  }

  return (
    <div className="relative flex h-screen flex-col">
      <AmbientBackground />

      <header className="relative z-10 flex h-14 shrink-0 items-center justify-between border-b border-line bg-canvas/70 px-5 backdrop-blur">
        <div className="flex items-center gap-2.5">
          <Wordmark />
          <div>
            <p className="text-[13px] font-semibold leading-none tracking-tight text-ink">
              Image Compressor
            </p>
            <p className="mt-1 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
              Target size
            </p>
          </div>
        </div>
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            onClick={() => setHelpOpen(true)}
            aria-label="How to use this app"
            className="grid h-8 w-8 place-items-center rounded-md border border-line bg-surface text-sm font-semibold text-muted transition-[color,border-color,transform] duration-150 hover:border-line-strong hover:text-ink active:scale-95"
          >
            ?
          </button>
          <ThemeToggle theme={theme} onToggle={toggleTheme} />
        </div>
      </header>

      <main className="relative z-10 mx-auto flex w-full max-w-6xl flex-1 flex-col gap-5 overflow-y-auto px-5 py-5 md:grid md:grid-cols-[minmax(0,1fr)_344px] md:overflow-hidden">
        <div className="flex min-h-[320px] flex-col gap-4 md:min-h-0 md:overflow-hidden">
          <SummaryBanner />
          <SamplePreview />
          <div className="min-h-[320px] flex-1 md:min-h-0">
            <Intake />
          </div>
        </div>

        <aside className="flex flex-col gap-4 md:min-h-0 md:overflow-y-auto md:pr-1">
          <Panel className="space-y-5 p-4">
            <PresetBar />
            <CapControls />
          </Panel>
          <Settings />
          <Panel className="sticky bottom-0 mt-auto p-4 shadow-panel">
            <RunBar />
          </Panel>
        </aside>
      </main>

      <HelpDialog open={helpOpen} onClose={() => setHelpOpen(false)} />
    </div>
  )
}
