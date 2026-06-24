import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import App from './App'
import { useStore, DEFAULT_SETTINGS } from './store/useStore'

beforeEach(() => {
  useStore.setState({
    inputs: [],
    results: {},
    phase: 'idle',
    completed: 0,
    total: 0,
    error: null,
    settings: { ...DEFAULT_SETTINGS },
  })
})

describe('App', () => {
  it('renders the header and the empty-state call to action', () => {
    render(<App />)
    expect(screen.getByText('Image Compressor')).toBeInTheDocument()
    expect(screen.getByText('Drop images or a folder here')).toBeInTheDocument()
    expect(screen.getByText('Add images to start')).toBeInTheDocument()
  })

  it('reflects queued inputs in the action button label', () => {
    useStore.getState().addInputs([
      { path: '/photo-1.jpg', bytes: 2_000_000 },
      { path: '/photo-2.jpg', bytes: 3_000_000 },
    ])
    render(<App />)
    expect(screen.getByText('photo-1.jpg')).toBeInTheDocument()
    // Outside Tauri the compress button is disabled, but its label still reflects the count.
    expect(screen.getByText('Compress 2 images')).toBeInTheDocument()
  })
})
