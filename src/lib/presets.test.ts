import { describe, it, expect } from 'vitest'
import { recipeOf, describeRecipe, DELIVERY_PRESETS } from './presets'
import { DEFAULT_SETTINGS, buildOptions } from '../store/useStore'

describe('presets', () => {
  it('recipeOf drops outputDir but preserves the recipe', () => {
    const s = { ...DEFAULT_SETTINGS, outputDir: '/tmp/out', capValue: 250 }
    const r = recipeOf(s)
    expect('outputDir' in r).toBe(false)
    expect(r.capValue).toBe(250)
  })

  it('a saved recipe round-trips to identical Options (B2 acceptance)', () => {
    const saved = {
      ...DEFAULT_SETTINGS,
      capValue: 300,
      outputFormat: 'png' as const,
      convertSrgb: true,
      perceptualFloorEnabled: true,
      perceptualFloorPct: 88,
      renamePattern: '{name}-{seq:000}',
    }
    // Apply the snapshotted recipe over fresh defaults, as loading a preset does.
    const loaded = { ...DEFAULT_SETTINGS, ...recipeOf(saved) }
    expect(buildOptions(loaded)).toEqual(buildOptions(saved))
  })

  it('delivery presets carry a caption and describeRecipe summarizes a patch', () => {
    for (const p of DELIVERY_PRESETS) expect(p.note.length).toBeGreaterThan(0)
    expect(
      describeRecipe({ outputFormat: 'avif', capValue: 200, capUnit: 'KB', capMode: 'perFile' }),
    ).toContain('AVIF')
  })
})
