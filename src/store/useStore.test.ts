import { describe, it, expect, beforeEach } from 'vitest'
import { useStore, buildOptions, DEFAULT_SETTINGS } from './useStore'

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

describe('buildOptions', () => {
  it('converts a KB cap to bytes and omits maxDimension when disabled', () => {
    const o = buildOptions({ ...DEFAULT_SETTINGS, capValue: 500, capUnit: 'KB', maxDimensionEnabled: false })
    expect(o.capBytes).toBe(500 * 1024)
    expect(o.maxDimension).toBeNull()
    expect(o.outputFormat).toBe('jpeg')
    expect(o.minLongEdge).toBe(16)
  })

  it('includes maxDimension when enabled and converts an MB cap', () => {
    const o = buildOptions({
      ...DEFAULT_SETTINGS,
      capValue: 2,
      capUnit: 'MB',
      maxDimensionEnabled: true,
      maxDimension: 1600,
    })
    expect(o.capBytes).toBe(2 * 1024 * 1024)
    expect(o.maxDimension).toBe(1600)
  })
})

describe('inputs', () => {
  it('dedupes by path (last write wins) and sorts', () => {
    useStore.getState().addInputs([
      { path: '/b.jpg', bytes: 10 },
      { path: '/a.jpg', bytes: 20 },
    ])
    useStore.getState().addInputs([{ path: '/b.jpg', bytes: 99 }])
    const inputs = useStore.getState().inputs
    expect(inputs.map((i) => i.path)).toEqual(['/a.jpg', '/b.jpg'])
    expect(inputs.find((i) => i.path === '/b.jpg')?.bytes).toBe(99)
  })

  it('removeInput drops a single file', () => {
    useStore.getState().addInputs([{ path: '/a.jpg', bytes: 1 }])
    useStore.getState().removeInput('/a.jpg')
    expect(useStore.getState().inputs).toHaveLength(0)
  })

  it('does not add inputs while a run is in progress', () => {
    useStore.setState({ phase: 'running' })
    useStore.getState().addInputs([{ path: '/a.jpg', bytes: 1 }])
    expect(useStore.getState().inputs).toHaveLength(0)
  })
})
