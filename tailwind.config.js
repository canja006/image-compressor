/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ['selector', '[data-theme="dark"]'],
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        canvas: 'rgb(var(--c-canvas) / <alpha-value>)',
        surface: 'rgb(var(--c-surface) / <alpha-value>)',
        sunken: 'rgb(var(--c-sunken) / <alpha-value>)',
        ink: 'rgb(var(--c-ink) / <alpha-value>)',
        muted: 'rgb(var(--c-muted) / <alpha-value>)',
        faint: 'rgb(var(--c-faint) / <alpha-value>)',
        line: 'rgb(var(--c-line) / <alpha-value>)',
        'line-strong': 'rgb(var(--c-line-strong) / <alpha-value>)',
        accent: 'rgb(var(--c-accent) / <alpha-value>)',
        'accent-fg': 'rgb(var(--c-accent-fg) / <alpha-value>)',
        good: 'rgb(var(--c-good) / <alpha-value>)',
        'good-bg': 'rgb(var(--c-good-bg) / <alpha-value>)',
        warn: 'rgb(var(--c-warn) / <alpha-value>)',
        'warn-bg': 'rgb(var(--c-warn-bg) / <alpha-value>)',
        bad: 'rgb(var(--c-bad) / <alpha-value>)',
        'bad-bg': 'rgb(var(--c-bad-bg) / <alpha-value>)',
        info: 'rgb(var(--c-info) / <alpha-value>)',
        'info-bg': 'rgb(var(--c-info-bg) / <alpha-value>)',
      },
      fontFamily: {
        sans: [
          '-apple-system',
          'BlinkMacSystemFont',
          '"SF Pro Text"',
          '"Segoe UI Variable Text"',
          '"Segoe UI"',
          'system-ui',
          'sans-serif',
        ],
        mono: [
          '"SF Mono"',
          'SFMono-Regular',
          '"Cascadia Code"',
          '"JetBrains Mono"',
          'ui-monospace',
          'Menlo',
          'Consolas',
          'monospace',
        ],
      },
      letterSpacing: {
        tightest: '-0.03em',
      },
      boxShadow: {
        subtle: '0 1px 2px rgba(15, 15, 14, 0.03)',
        lift: '0 2px 10px rgba(15, 15, 14, 0.05)',
        panel: '0 1px 0 rgba(15, 15, 14, 0.02), 0 8px 24px -16px rgba(15, 15, 14, 0.10)',
      },
      keyframes: {
        'fade-up': {
          '0%': { opacity: '0', transform: 'translateY(8px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        'fade-in': {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        indeterminate: {
          '0%': { transform: 'translateX(-100%)' },
          '100%': { transform: 'translateX(400%)' },
        },
        drift: {
          '0%, 100%': { transform: 'translate(0, 0)' },
          '50%': { transform: 'translate(3%, -2%)' },
        },
      },
      animation: {
        'fade-up': 'fade-up 460ms cubic-bezier(0.16, 1, 0.3, 1) both',
        'fade-in': 'fade-in 320ms ease both',
        indeterminate: 'indeterminate 1.1s ease-in-out infinite',
        drift: 'drift 24s ease-in-out infinite',
      },
    },
  },
  plugins: [],
}
