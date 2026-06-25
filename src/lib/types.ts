// TypeScript mirror of the engine's serde types (src-tauri/crates/engine/src/model.rs).
// Field names are camelCase to match `#[serde(rename_all = "camelCase")]`.

export type OutputFormat = 'keep' | 'jpeg' | 'png' | 'avif'

export type CollisionPolicy = 'suffix' | 'overwrite' | 'skip'

/** How EXIF/metadata is handled on output (mirrors the Rust `MetadataMode`). Orientation is always
 *  baked into the pixels regardless; this controls what metadata is re-embedded. */
export type MetadataMode = 'stripAll' | 'keepAll' | 'keepOrientationIcc' | 'stripGps'

/** Which edge a crop is anchored to on the cropped axis when producing an exact size. */
export type Anchor = 'start' | 'center' | 'end'

/** How the engine sizes the output before the byte-cap search (mirrors the Rust `ResizeMode`). */
export type ResizeMode =
  | { mode: 'fit'; maxDimension: number | null }
  | { mode: 'exact'; width: number; height: number; anchor: Anchor; allowUpscale: boolean }

export interface Options {
  capBytes: number
  resize: ResizeMode
  outputFormat: OutputFormat
  outputDir: string | null
  suffix: string
  collision: CollisionPolicy
  stripMetadata: boolean
  skipIfUnderCap: boolean
  jpegQualityMin: number
  jpegQualityMax: number
  minLongEdge: number
  background: [number, number, number]
  /** How EXIF/metadata is re-embedded on output. */
  metadata: MetadataMode
  /** Convert pixels to sRGB (via the source ICC) before encoding. */
  convertSrgb: boolean
  /** Optional SSIM floor (0.0–1.0): the search won't ship below this fidelity, trading resolution
   *  instead. `null` disables it. */
  perceptualFloor: number | null
  /** Optional output-name pattern with tokens ({name}, {seq:000}, {date}, {w}, {h}). `null` keeps
   *  the default stem + suffix naming. */
  renamePattern: string | null
}

export type Outcome =
  | {
      kind: 'compressed'
      finalBytes: number
      quality: number | null
      width: number
      height: number
      downscaled: boolean
    }
  | { kind: 'skippedUnderCap'; bytes: number }
  | { kind: 'skippedCollision' }
  | { kind: 'unreachable'; reason: string }
  | { kind: 'failed'; reason: string }
  | { kind: 'cancelled' }

export interface FileResult {
  input: string
  output: string | null
  originalBytes: number
  outcome: Outcome
}

export interface BatchSummary {
  results: FileResult[]
  cancelled: boolean
}

export interface Progress {
  completed: number
  total: number
  result: FileResult
}

export interface InputFile {
  path: string
  bytes: number
}

/** A file to compress, with an optional per-file cap overriding the batch default. */
export interface BatchItem {
  path: string
  capOverride: number | null
}

/** Result of an in-memory single-image preview (the `preview_sample` command). */
export interface Preview {
  originalBytes: number
  sourceWidth: number
  sourceHeight: number
  hasAlpha: boolean
  kind: 'compressed' | 'unreachable' | 'failed'
  finalBytes: number | null
  quality: number | null
  width: number | null
  height: number | null
  downscaled: boolean
  /** True when finalBytes is an estimate extrapolated from a downscaled search (large images). */
  approx: boolean
  mime: string | null
  error: string | null
  /** SSIM (0–1) and PSNR (dB) of the result vs the source — the B6 quality readout. `null` when not
   *  computable (AVIF) or non-finite (lossless PSNR serializes to null). */
  ssim: number | null
  psnr: number | null
  /** Data URL of the compressed result, ready for an `<img src>` (null unless compressed). */
  dataUrl: string | null
}

/** Lightweight predicted compressed size for a file-list row (the `estimate_size` command). */
export interface SizeEstimate {
  kind: 'compressed' | 'unreachable' | 'failed'
  finalBytes: number | null
  /** True when the size is extrapolated from a downscaled search (large images). */
  approx: boolean
}
