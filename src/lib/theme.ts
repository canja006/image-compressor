export type Theme = 'light' | 'dark'

const THEME_KEY = 'image-compressor.theme'

/** Resolve the initial theme: stored preference first, then the OS setting, else light. */
export function getInitialTheme(): Theme {
  try {
    const stored = localStorage.getItem(THEME_KEY)
    if (stored === 'light' || stored === 'dark') return stored
  } catch {
    // ignore storage failures
  }
  if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
    if (window.matchMedia('(prefers-color-scheme: dark)').matches) return 'dark'
  }
  return 'light'
}

/** Apply a theme to the document root and persist the choice. */
export function applyTheme(theme: Theme): void {
  if (typeof document !== 'undefined') {
    document.documentElement.dataset.theme = theme
  }
  try {
    localStorage.setItem(THEME_KEY, theme)
  } catch {
    // ignore storage failures
  }
}
