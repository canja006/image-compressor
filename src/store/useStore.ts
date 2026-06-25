import { create } from 'zustand'
import type {
  Anchor,
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

/** Whether the cap is a per-image target or a combined budget split across all images. */
export type CapMode = 'perFile' | 'totalBudget'

/** UI-facing settings. `capValue`/`capUnit` are split for the KB/MB toggle; everything else maps
 *  to the engine `Options` via {@link buildOptions}. */
export interface Settings {
  capValue: number
  capUnit: SizeUnit
  capMode: CapMode
  /** 'fit' preserves aspect ratio (longest-edge cap); 'exact' crops to an exact width × height. */
  resizeMode: 'fit' | 'exact'
  maxDimensionEnabled: boolean
  maxDimension: number
  exactWidth: number
  exactHeight: number
  exactAnchor: Anchor
  exactAllowUpscale: boolean
  outputFormat: OutputFormat
  outputDir: string | null
  suffix: string
  collision: CollisionPolicy
  skipIfUnderCap: boolean
  jpegQualityMin: number
  jpegQualityMax: number
  /** Fill color for transparent areas when flattening to JPEG. */
  background: [number, number, number]
}

export const DEFAULT_SETTINGS: Settings = {
  capValue: 500,
  capUnit: 'KB',
  capMode: 'perFile',
  resizeMode: 'fit',
  maxDimensionEnabled: false,
  maxDimension: 2000,
  exactWidth: 1920,
  exactHeight: 1080,
  exactAnchor: 'center',
  exactAllowUpscale: true,
  outputFormat: 'jpeg',
  outputDir: null,
  suffix: '-compressed',
  collision: 'suffix',
  skipIfUnderCap: true,
  jpegQualityMin: 10,
  jpegQualityMax: 95,
  background: [255, 255, 255],
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
    resize:
      s.resizeMode === 'exact'
        ? {
            mode: 'exact',
            width: Math.max(1, Math.floor(s.exactWidth)),
            height: Math.max(1, Math.floor(s.exactHeight)),
            anchor: s.exactAnchor,
            allowUpscale: s.exactAllowUpscale,
          }
        : {
            mode: 'fit',
            maxDimension: s.maxDimensionEnabled ? Math.max(1, Math.floor(s.maxDimension)) : null,
          },
    outputFormat: s.outputFormat,
    outputDir: s.outputDir,
    suffix: s.suffix,
    collision: s.collision,
    stripMetadata: true,
    skipIfUnderCap: s.skipIfUnderCap,
    jpegQualityMin: s.jpegQualityMin,
    jpegQualityMax: s.jpegQualityMax,
    minLongEdge: 16,
    background: s.background,
  }
}

interface StoreState {
  inputs: InputFile[]
  results: Record<string, FileResult>
  /** Per-file size-cap overrides in bytes, keyed by path. Absent = use the batch cap. */
  capOverrides: Record<string, number>
  /** Which file the preview shows. `null` falls back to the first input. */
  selectedPath: string | null
  /** Source pixel dimensions of the previewed image, for the live exact-crop note. */
  previewSource: { width: number; height: number } | null
  phase: Phase
  completed: number
  total: number
  error: string | null
  settings: Settings

  addInputs: (files: InputFile[]) => void
  removeInput: (path: string) => void
  clearInputs: () => void
  selectPreview: (path: string) => void
  setPreviewSource: (dims: { width: number; height: number } | null) => void
  setCapOverride: (path: string, bytes: number | null) => void
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
  capOverrides: {},
  selectedPath: null,
  previewSource: null,
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
      const capOverrides = { ...state.capOverrides }
      delete capOverrides[path]
      return {
        inputs: state.inputs.filter((f) => f.path !== path),
        results,
        capOverrides,
        selectedPath: state.selectedPath === path ? null : state.selectedPath,
      }
    }),

  clearInputs: () =>
    set({
      inputs: [],
      results: {},
      capOverrides: {},
      selectedPath: null,
      phase: 'idle',
      completed: 0,
      total: 0,
      error: null,
    }),

  selectPreview: (path) => set({ selectedPath: path }),

  setPreviewSource: (dims) => set({ previewSource: dims }),

  setCapOverride: (path, bytes) =>
    set((state) => {
      const capOverrides = { ...state.capOverrides }
      if (bytes == null || bytes <= 0) delete capOverrides[path]
      else capOverrides[path] = Math.floor(bytes)
      return { capOverrides }
    }),

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
      // Parallel workers can emit out of order; never let the counter go backwards.
      completed: Math.max(state.completed, p.completed),
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
