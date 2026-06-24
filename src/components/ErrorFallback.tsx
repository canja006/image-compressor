import { Component, type ErrorInfo, type ReactNode } from 'react'
import { Button } from './ui'
import { IconAlert } from '../lib/icons'

export function ErrorFallback({ error, onReset }: { error: Error; onReset: () => void }) {
  return (
    <div className="grid h-screen place-items-center bg-canvas px-6">
      <div className="max-w-md space-y-4 text-center">
        <span className="mx-auto grid h-12 w-12 place-items-center rounded-xl border border-line bg-bad-bg text-bad">
          <IconAlert size={22} />
        </span>
        <div className="space-y-1.5">
          <h1 className="text-lg font-semibold text-ink">Something went wrong</h1>
          <p className="text-sm text-muted">{error.message || 'An unexpected error occurred.'}</p>
        </div>
        <Button variant="primary" onClick={onReset}>
          Reload the view
        </Button>
      </div>
    </div>
  )
}

interface BoundaryProps {
  children: ReactNode
}
interface BoundaryState {
  error: Error | null
}

/** Catches render-time errors so a component bug shows a recoverable screen, not a blank window. */
export class ErrorBoundary extends Component<BoundaryProps, BoundaryState> {
  state: BoundaryState = { error: null }

  static getDerivedStateFromError(error: Error): BoundaryState {
    return { error }
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error('UI error boundary caught:', error, info.componentStack)
  }

  render(): ReactNode {
    if (this.state.error) {
      return <ErrorFallback error={this.state.error} onReset={() => this.setState({ error: null })} />
    }
    return this.props.children
  }
}
