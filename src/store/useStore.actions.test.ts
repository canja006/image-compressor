import { describe, it, expect, beforeEach } from 'vitest'
import { useStore, DEFAULT_SETTINGS } from './useStore'
import type { BatchSummary, FileResult, Progress } from '../lib/types'

beforeEach(() => {
  localStorage.clear()
  useStore.setState({
    inputs: [],
    results: {},
    capOverrides: {},
    selectedPath: null,
    phase: 'idle',
    completed: 0,
    total: 0,
    error: null,
    settings: { ...DEFAULT_SETTINGS },
  })
})

const compressed = (path: string): FileResult => ({
  input: path,
  output: `${path}.out`,
  originalBytes: 1000,
  outcome: { kind: 'compressed', finalBytes: 400, quality: 70, width: 10, height: 10, downscaled: false },
})

const progress = (path: string, completed: number, total: number): Progress => ({
  completed,
  total,
  result: compressed(path),
})

describe('run lifecycle', () => {
  it('begins, accumulates progress, and finalizes', () => {
    useStore.getState().addInputs([
      { path: '/a.jpg', bytes: 1000 },
      { path: '/b.jpg', bytes: 2000 },
    ])
    useStore.getState().beginRun()
    expect(useStore.getState().phase).toBe('running')
    expect(useStore.getState().total).toBe(2)

    useStore.getState().recordProgress(progress('/a.jpg', 1, 2))
    expect(useStore.getState().completed).toBe(1)
    expect(useStore.getState().results['/a.jpg'].outcome.kind).toBe('compressed')

    const summary: BatchSummary = {
      cancelled: false,
      results: [
        compressed('/a.jpg'),
        { input: '/b.jpg', output: null, originalBytes: 2000, outcome: { kind: 'failed', reason: 'x' } },
      ],
    }
    useStore.getState().endRun(summary)
    expect(useStore.getState().phase).toBe('done')
    expect(useStore.getState().results['/b.jpg'].outcome.kind).toBe('failed')
  })

  it('setError marks the run done with a message', () => {
    useStore.getState().setError('boom')
    expect(useStore.getState().phase).toBe('done')
    expect(useStore.getState().error).toBe('boom')
  })

  it('resetRun clears results and the error and returns to idle', () => {
    useStore.getState().setError('x')
    useStore.getState().resetRun()
    expect(useStore.getState().phase).toBe('idle')
    expect(useStore.getState().error).toBeNull()
    expect(useStore.getState().results).toEqual({})
  })
})

describe('settings persistence', () => {
  it('updateSettings updates state and writes to localStorage', () => {
    useStore.getState().updateSettings({ capValue: 250, capUnit: 'MB' })
    expect(useStore.getState().settings.capValue).toBe(250)
    expect(useStore.getState().settings.capUnit).toBe('MB')
    expect(localStorage.getItem('image-compressor.settings')).toContain('250')
  })
})

describe('per-file cap overrides', () => {
  it('sets and floors a value, and clears on null or non-positive', () => {
    const s = () => useStore.getState()
    s().setCapOverride('/a.jpg', 1024 * 1024)
    expect(s().capOverrides['/a.jpg']).toBe(1024 * 1024)
    s().setCapOverride('/a.jpg', 500.7)
    expect(s().capOverrides['/a.jpg']).toBe(500)
    s().setCapOverride('/a.jpg', null)
    expect(s().capOverrides['/a.jpg']).toBeUndefined()
    s().setCapOverride('/a.jpg', 0)
    expect(s().capOverrides['/a.jpg']).toBeUndefined()
  })

  it('selectPreview sets the path and removing that input clears the selection', () => {
    useStore.getState().addInputs([
      { path: '/a.jpg', bytes: 1 },
      { path: '/b.jpg', bytes: 2 },
    ])
    useStore.getState().selectPreview('/b.jpg')
    expect(useStore.getState().selectedPath).toBe('/b.jpg')
    useStore.getState().removeInput('/b.jpg')
    expect(useStore.getState().selectedPath).toBeNull()
  })

  it('removeInput drops the override and clearInputs wipes all', () => {
    useStore.getState().addInputs([
      { path: '/a.jpg', bytes: 1 },
      { path: '/b.jpg', bytes: 2 },
    ])
    useStore.getState().setCapOverride('/a.jpg', 2048)
    useStore.getState().setCapOverride('/b.jpg', 4096)
    useStore.getState().removeInput('/a.jpg')
    expect(useStore.getState().capOverrides).toEqual({ '/b.jpg': 4096 })
    useStore.getState().clearInputs()
    expect(useStore.getState().capOverrides).toEqual({})
  })
})
