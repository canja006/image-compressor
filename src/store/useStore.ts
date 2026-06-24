import { create } from 'zustand'
import type {
  BatchSummary,
  CollisionPolicy,
  FileResult,
  InputFile,
  Options,
  OutputFormat,
  Progress,
} from '../lib/types'
import { parseSizeToBytes, type SizeUnit } from '../lib/format'

export type Phase = 'idle' | 'running' | 'done'

/** UI-facing settings. `capValue`/`capUnit` are split for the KB/MB toggle; everything else maps
 *  to the engine `Options` via {@link buildOptions}. */
export interface Settings {
  capValue: number
  capUnit: SizeUnit
  maxDimensionEnabled: boolean
  maxDimension: number
  outputFormat: OutputFormat
  outputDir: string | null
  suffix: string
  collision: CollisionPolicy
  skipIfUnderCap: boolean
  jpegQualityMin: number
  jpegQualityMax: number
}

export const DEFAULT_SETTINGS: Settings = {
  capValue: 500,
  capUnit: 'KB',
  maxDimensionEnabled: false,
  maxDimension: 2000,
  outputFormat: 'jpeg',
  outputDir: null,
  suffix: '-compressed',
  collision: 'suffix',
  skipIfUnderCap: true,
  jpegQualityMin: 10,
  jpegQualityMax: 95,
}

const SETTINGS_KEY = 'image-compressor.settings'

function loadSettings(): Settings {
  try {
    const raw = typeof localStorage !== 'undefined' && localStorage.getItem(SETTINGS_KEY)
    if (!raw) return DEFAULT_SETTINGS
    const parsed = JSON.parse(raw) as Partial<Settings>
    return { ...DEFAULT_SETTINGS, ...parsed }
  } catch {
    return DEFAULT_SETTINGS
  }
}

function persistSettings(settings: Settings): void {
  try {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings))
    }
  } catch {
    // Persistence is best-effort; ignore quota/private-mode failures.
  }
}

/** Translate UI settings into the exact `Options` the engine expects. */
export function buildOptions(s: Settings): Options {
  return {
    capBytes: parseSizeToBytes(s.capValue, s.capUnit),
    maxDimension: s.maxDimensionEnabled ? Math.max(1, Math.floor(s.maxDimension)) : null,
    outputFormat: s.outputFormat,
    outputDir: s.outputDir,
    suffix: s.suffix,
    collision: s.collision,
    stripMetadata: true,
    skipIfUnderCap: s.skipIfUnderCap,
    jpegQualityMin: s.jpegQualityMin,
    jpegQualityMax: s.jpegQualityMax,
    minLongEdge: 16,
    background: [255, 255, 255],
  }
}

interface StoreState {
  inputs: InputFile[]
  results: Record<string, FileResult>
  phase: Phase
  completed: number
  total: number
  error: string | null
  settings: Settings

  addInputs: (files: InputFile[]) => void
  removeInput: (path: string) => void
  clearInputs: () => void
  updateSettings: (patch: Partial<Settings>) => void
  beginRun: () => void
  recordProgress: (p: Progress) => void
  endRun: (summary: BatchSummary) => void
  setError: (message: string) => void
  resetRun: () => void
}

export const useStore = create<StoreState>((set, get) => ({
  inputs: [],
  results: {},
  phase: 'idle',
  completed: 0,
  total: 0,
  error: null,
  settings: loadSettings(),

  addInputs: (files) =>
    set((state) => {
      if (state.phase === 'running') return state
      const byPath = new Map(state.inputs.map((f) => [f.path, f]))
      for (const f of files) byPath.set(f.path, f)
      const inputs = Array.from(byPath.values()).sort((a, b) => a.path.localeCompare(b.path))
      // Adding files after a run starts a fresh session.
      return { inputs, results: {}, phase: 'idle', completed: 0, total: 0, error: null }
    }),

  removeInput: (path) =>
    set((state) => {
      if (state.phase === 'running') return state
      const results = { ...state.results }
      delete results[path]
      return { inputs: state.inputs.filter((f) => f.path !== path), results }
    }),

  clearInputs: () =>
    set({ inputs: [], results: {}, phase: 'idle', completed: 0, total: 0, error: null }),

  updateSettings: (patch) =>
    set((state) => {
      const settings = { ...state.settings, ...patch }
      persistSettings(settings)
      return { settings }
    }),

  beginRun: () =>
    set((state) => ({
      phase: 'running',
      results: {},
      completed: 0,
      total: state.inputs.length,
      error: null,
    })),

  recordProgress: (p) =>
    set((state) => ({
      results: { ...state.results, [p.result.input]: p.result },
      completed: p.completed,
      total: p.total,
    })),

  endRun: (summary) =>
    set((state) => {
      const results = { ...state.results }
      for (const r of summary.results) results[r.input] = r
      return { phase: 'done', results, completed: get().total || summary.results.length }
    }),

  setError: (message) => set({ error: message, phase: 'done' }),

  resetRun: () => set({ phase: 'idle', results: {}, completed: 0, total: 0, error: null }),
}))
