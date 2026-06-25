// Named recipes (B2) and built-in delivery presets (B4). A preset is a patch of recipe-relevant
// settings (everything except the per-session output folder) applied over the current settings.

import type { Settings } from '../store/useStore'

/** A patch of settings a preset applies. Never includes `outputDir` (kept per-session). */
export type RecipePatch = Partial<Settings>

export interface Preset {
  id: string
  name: string
  patch: RecipePatch
  /** A short caption shown under the name (e.g. the cap + format). */
  note: string
  /** True for the shipped delivery presets (not user-editable / not deletable). */
  builtin?: boolean
}

/** Snapshot the current settings as a preset patch, dropping the output folder. */
export function recipeOf(s: Settings): RecipePatch {
  const rest: RecipePatch = { ...s }
  delete rest.outputDir
  return rest
}

/** Built-in delivery presets (B4). Each sets the full recipe needed for that delivery target. */
export const DELIVERY_PRESETS: ReadonlyArray<Preset> = [
  {
    id: 'builtin-mls',
    name: 'MLS listing',
    note: 'JPEG ≤ 2 MB · 2048px · sRGB · no GPS',
    builtin: true,
    patch: {
      capMode: 'perFile',
      capValue: 2,
      capUnit: 'MB',
      resizeMode: 'fit',
      maxDimensionEnabled: true,
      maxDimension: 2048,
      outputFormat: 'jpeg',
      convertSrgb: true,
      metadata: 'stripGps',
    },
  },
  {
    id: 'builtin-web-hero',
    name: 'Web hero',
    note: 'AVIF ≤ 200 KB · 1920px · sRGB',
    builtin: true,
    patch: {
      capMode: 'perFile',
      capValue: 200,
      capUnit: 'KB',
      resizeMode: 'fit',
      maxDimensionEnabled: true,
      maxDimension: 1920,
      outputFormat: 'avif',
      convertSrgb: true,
      metadata: 'stripAll',
    },
  },
  {
    id: 'builtin-gallery',
    name: 'Client gallery',
    note: 'Whole shoot under a 50 MB budget',
    builtin: true,
    patch: {
      capMode: 'totalBudget',
      capValue: 50,
      capUnit: 'MB',
      resizeMode: 'fit',
      maxDimensionEnabled: true,
      maxDimension: 2560,
      outputFormat: 'jpeg',
      convertSrgb: true,
      metadata: 'stripAll',
    },
  },
]

const STORAGE_KEY = 'image-compressor.presets'

/** Load user-saved presets from localStorage (best-effort; never throws). */
export function loadUserPresets(): Preset[] {
  try {
    const raw = typeof localStorage !== 'undefined' && localStorage.getItem(STORAGE_KEY)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return parsed.filter(
      (p): p is Preset =>
        p && typeof p.id === 'string' && typeof p.name === 'string' && typeof p.patch === 'object',
    )
  } catch {
    return []
  }
}

/** Persist user presets (best-effort; ignores quota/private-mode failures). */
export function saveUserPresets(presets: Preset[]): void {
  try {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(presets))
    }
  } catch {
    // ignore
  }
}

/** A short, stable id for a new user preset. */
export function newPresetId(): string {
  try {
    if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID()
  } catch {
    // fall through
  }
  return `p-${Date.now().toString(36)}-${Math.floor(Math.random() * 1e6).toString(36)}`
}

/** A concise caption summarizing a saved recipe (cap + format). */
export function describeRecipe(p: RecipePatch): string {
  const cap = p.capValue != null && p.capUnit ? `${p.capValue} ${p.capUnit}` : 'custom cap'
  const fmt = (p.outputFormat ?? 'jpeg').toUpperCase()
  const scope = p.capMode === 'totalBudget' ? 'budget' : 'per file'
  return `${fmt} · ${cap} ${scope}`
}
