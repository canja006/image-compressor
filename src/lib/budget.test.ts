import { describe, it, expect } from 'vitest'
import { splitBudget } from './budget'
import type { InputFile } from './types'

const f = (path: string, bytes: number): InputFile => ({ path, bytes })

describe('splitBudget', () => {
  it('splits proportionally to original size and never exceeds the budget', () => {
    const inputs = [f('/a', 1000), f('/b', 3000)] // total 4000
    const budget = 800
    const caps = splitBudget(inputs, budget)
    expect(caps['/a']).toBe(Math.floor((800 * 1000) / 4000)) // 200
    expect(caps['/b']).toBe(Math.floor((800 * 3000) / 4000)) // 600
    expect(caps['/a'] + caps['/b']).toBeLessThanOrEqual(budget)
  })

  it('returns an empty map for no inputs or a non-positive budget', () => {
    expect(splitBudget([], 1000)).toEqual({})
    expect(splitBudget([f('/a', 10)], 0)).toEqual({})
  })

  it('gives every file at least one byte', () => {
    const caps = splitBudget([f('/a', 1), f('/b', 1_000_000)], 100)
    expect(caps['/a']).toBeGreaterThanOrEqual(1)
  })
})
