import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { CapControls } from './CapControls'
import { Settings } from './Settings'
import { HelpDialog } from './HelpDialog'
import { useStore, DEFAULT_SETTINGS, buildOptions } from '../store/useStore'

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

describe('AVIF output format', () => {
  it('can be selected and flows into the engine options', async () => {
    const user = userEvent.setup()
    render(<CapControls />)
    await user.click(screen.getByRole('radio', { name: 'AVIF' }))
    expect(useStore.getState().settings.outputFormat).toBe('avif')
    expect(buildOptions(useStore.getState().settings).outputFormat).toBe('avif')
  })
})

describe('JPEG background control', () => {
  it('appears for JPEG and updates the background setting', async () => {
    const user = userEvent.setup()
    render(<Settings />)
    await user.click(screen.getByText('Output & advanced'))
    await user.click(screen.getByRole('button', { name: 'Use background #000000' }))
    expect(useStore.getState().settings.background).toEqual([0, 0, 0])
  })

  it('is hidden when the output format is PNG', async () => {
    const user = userEvent.setup()
    useStore.setState({ settings: { ...DEFAULT_SETTINGS, outputFormat: 'png' } })
    render(<Settings />)
    await user.click(screen.getByText('Output & advanced'))
    expect(screen.queryByLabelText('JPEG background color')).toBeNull()
  })
})

describe('HelpDialog', () => {
  it('is hidden when closed and shows the instructions when open', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    const { rerender } = render(<HelpDialog open={false} onClose={onClose} />)
    expect(screen.queryByRole('dialog')).toBeNull()

    rerender(<HelpDialog open onClose={onClose} />)
    expect(screen.getByRole('dialog')).toBeInTheDocument()
    expect(screen.getByText('Add images')).toBeInTheDocument()
    expect(screen.getByText('Set the target size')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Got it' }))
    expect(onClose).toHaveBeenCalled()
  })
})
