// TypeScript mirror of the engine's serde types (src-tauri/crates/engine/src/model.rs).
// Field names are camelCase to match `#[serde(rename_all = "camelCase")]`.

export type OutputFormat = 'keep' | 'jpeg' | 'png'

export type CollisionPolicy = 'suffix' | 'overwrite' | 'skip'

export interface Options {
  capBytes: number
  maxDimension: number | null
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
