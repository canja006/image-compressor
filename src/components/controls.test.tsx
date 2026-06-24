import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { CapControls } from './CapControls'
import { PresetBar } from './PresetBar'
import { Settings } from './Settings'
import { SummaryBanner } from './SummaryBanner'
import { RunBar } from './RunBar'
import { FileRow } from './FileRow'
import { useStore, DEFAULT_SETTINGS } from '../store/useStore'
import type { FileResult } from '../lib/types'

beforeEach(() => {
  localStorage.clear()
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

describe('PresetBar', () => {
  it('applies a preset to the store', async () => {
    const user = userEvent.setup()
    render(<PresetBar />)
    await user.click(screen.getByRole('button', { name: /Portal/ }))
    expect(useStore.getState().settings.capValue).toBe(2)
    expect(useStore.getState().settings.capUnit).toBe('MB')
  })
})

describe('CapControls', () => {
  it('switches the output format', async () => {
    const user = userEvent.setup()
    render(<CapControls />)
    await user.click(screen.getByRole('radio', { name: 'PNG' }))
    expect(useStore.getState().settings.outputFormat).toBe('png')
  })

  it('toggles the dimension limit and reveals the pixel field', async () => {
    const user = userEvent.setup()
    render(<CapControls />)
    expect(screen.queryByLabelText('Maximum longest edge in pixels')).toBeNull()
    await user.click(screen.getByRole('switch', { name: 'Limit the longest edge' }))
    expect(useStore.getState().settings.maxDimensionEnabled).toBe(true)
    expect(screen.getByLabelText('Maximum longest edge in pixels')).toBeInTheDocument()
  })
})

describe('Settings', () => {
  it('expands and changes the collision policy', async () => {
    const user = userEvent.setup()
    render(<Settings />)
    await user.click(screen.getByText('Output & advanced'))
    await user.click(screen.getByRole('radio', { name: 'Overwrite' }))
    expect(useStore.getState().settings.collision).toBe('overwrite')
  })
})

describe('SummaryBanner', () => {
  it('renders nothing until the run is done', () => {
    const { container } = render(<SummaryBanner />)
    expect(container).toBeEmptyDOMElement()
  })

  it('summarizes results when the run is done', () => {
    useStore.setState({
      phase: 'done',
      results: {
        '/a.jpg': {
          input: '/a.jpg',
          output: '/a-c.jpg',
          originalBytes: 1000,
          outcome: { kind: 'compressed', finalBytes: 400, quality: 70, width: 10, height: 10, downscaled: false },
        },
        '/b.jpg': {
          input: '/b.jpg',
          output: null,
          originalBytes: 500,
          outcome: { kind: 'failed', reason: 'bad' },
        },
      },
    })
    render(<SummaryBanner />)
    expect(screen.getByText(/saved/i)).toBeInTheDocument()
    expect(screen.getByText('1 compressed')).toBeInTheDocument()
    expect(screen.getByText('1 failed')).toBeInTheDocument()
  })
})

describe('RunBar', () => {
  it('shows progress and a cancel button while running', () => {
    useStore.setState({ phase: 'running', completed: 2, total: 5 })
    render(<RunBar />)
    expect(screen.getByText(/Compressing/)).toBeInTheDocument()
    expect(screen.getByText('2 / 5')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument()
  })
})

describe('FileRow', () => {
  it('shows the original and final sizes plus the file name', () => {
    const input = { path: '/photos/cat.jpg', bytes: 2_000_000 }
    const result: FileResult = {
      input: '/photos/cat.jpg',
      output: '/photos/cat-c.jpg',
      originalBytes: 2_000_000,
      outcome: { kind: 'compressed', finalBytes: 500_000, quality: 62, width: 1600, height: 1200, downscaled: false },
    }
    render(
      <ul>
        <FileRow input={input} result={result} phase="done" index={0} onRemove={() => {}} />
      </ul>,
    )
    expect(screen.getByText('cat.jpg')).toBeInTheDocument()
    expect(screen.getByText('1.9 MB')).toBeInTheDocument()
    expect(screen.getByText('488.3 KB')).toBeInTheDocument()
  })
})
