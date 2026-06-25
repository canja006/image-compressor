import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { CapControls } from './CapControls'
import { useStore, DEFAULT_SETTINGS } from '../store/useStore'

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

describe('CapControls — exact mode', () => {
  it('hides exact-size fields in fit mode by default', () => {
    render(<CapControls />)
    expect(screen.queryByLabelText('Exact width in pixels')).toBeNull()
  })

  it('switches to exact mode and reveals width/height fields', async () => {
    const user = userEvent.setup()
    render(<CapControls />)
    await user.click(screen.getByRole('radio', { name: 'Exact' }))
    expect(useStore.getState().settings.resizeMode).toBe('exact')
    expect(screen.getByLabelText('Exact width in pixels')).toBeInTheDocument()
    expect(screen.getByLabelText('Exact height in pixels')).toBeInTheDocument()
  })

  it('edits the exact width', async () => {
    const user = userEvent.setup()
    render(<CapControls />)
    await user.click(screen.getByRole('radio', { name: 'Exact' }))
    const widthInput = screen.getByLabelText('Exact width in pixels')
    // fireEvent.change sets the controlled number input in one deterministic event (userEvent's
    // clear-then-type leaves the field at its min and appends, yielding the wrong value).
    fireEvent.change(widthInput, { target: { value: '800' } })
    expect(useStore.getState().settings.exactWidth).toBe(800)
  })

  it('changes the crop anchor', async () => {
    const user = userEvent.setup()
    render(<CapControls />)
    await user.click(screen.getByRole('radio', { name: 'Exact' }))
    await user.click(screen.getByRole('radio', { name: 'Start' }))
    expect(useStore.getState().settings.exactAnchor).toBe('start')
  })
})