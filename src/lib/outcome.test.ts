import { describe, it, expect } from 'vitest'
import { basename, describeOutcome, finalBytesOf } from './outcome'
import type { FileResult, Outcome } from './types'

function result(outcome: Outcome, originalBytes = 1000): FileResult {
  return { input: '/x.jpg', output: null, originalBytes, outcome }
}

describe('basename', () => {
  it('handles posix and windows separators', () => {
    expect(basename('/a/b/c.jpg')).toBe('c.jpg')
    expect(basename('C:\\a\\b\\c.png')).toBe('c.png')
    expect(basename('plain.webp')).toBe('plain.webp')
  })
})

describe('describeOutcome', () => {
  it('compressed shows percent saved, dimensions, and quality', () => {
    const v = describeOutcome(
      result({ kind: 'compressed', finalBytes: 250, quality: 70, width: 800, height: 600, downscaled: false }),
    )
    expect(v.tone).toBe('good')
    expect(v.label).toContain('75%')
    expect(v.detail).toContain('800×600')
    expect(v.detail).toContain('q70')
  })

  it('notes downscaling and omits quality when absent', () => {
    const v = describeOutcome(
      result({ kind: 'compressed', finalBytes: 900, quality: null, width: 100, height: 100, downscaled: true }),
    )
    expect(v.detail).toContain('downscaled')
    expect(v.detail).not.toContain('q1')
  })

  it('maps each non-compressed kind to the right tone', () => {
    expect(describeOutcome(result({ kind: 'skippedUnderCap', bytes: 10 })).tone).toBe('info')
    expect(describeOutcome(result({ kind: 'skippedCollision' })).tone).toBe('warn')
    expect(describeOutcome(result({ kind: 'unreachable', reason: 'too small' })).tone).toBe('bad')
    expect(describeOutcome(result({ kind: 'failed', reason: 'broken' })).tone).toBe('bad')
    expect(describeOutcome(result({ kind: 'cancelled' })).tone).toBe('neutral')
  })
})

describe('finalBytesOf', () => {
  it('returns a size for compressed and skipped-under-cap, else null', () => {
    expect(
      finalBytesOf(result({ kind: 'compressed', finalBytes: 42, quality: 50, width: 1, height: 1, downscaled: false })),
    ).toBe(42)
    expect(finalBytesOf(result({ kind: 'skippedUnderCap', bytes: 7 }))).toBe(7)
    expect(finalBytesOf(result({ kind: 'failed', reason: 'x' }))).toBeNull()
    expect(finalBytesOf(result({ kind: 'unreachable', reason: 'x' }))).toBeNull()
  })
})
