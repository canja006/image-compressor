import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import { ErrorBoundary } from './components/ErrorFallback'
import { applyTheme, getInitialTheme } from './lib/theme'
import './index.css'

// Apply the theme before first paint to avoid a flash of the wrong palette.
applyTheme(getInitialTheme())

const container = document.getElementById('root')
if (container) {
  ReactDOM.createRoot(container).render(
    <React.StrictMode>
      <ErrorBoundary>
        <App />
      </ErrorBoundary>
    </React.StrictMode>,
  )
}
