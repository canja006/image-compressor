// Map an engine outcome to its UI presentation (badge tone, short label, detail line).

import type { BadgeTone } from '../components/ui'
import { percentSaved } from './format'
import type { FileResult } from './types'

export interface OutcomeView {
  tone: BadgeTone
  label: string
  detail: string
}

/** Basename of a path, handling both `/` and `\` separators. */
export function basename(path: string): string {
  const i = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'))
  return i >= 0 ? path.slice(i + 1) : path
}

export function describeOutcome(result: FileResult): OutcomeView {
  const { outcome, originalBytes } = result
  switch (outcome.kind) {
    case 'compressed': {
      const saved = percentSaved(originalBytes, outcome.finalBytes)
      const bits = [`${outcome.width}×${outcome.height}`]
      if (outcome.quality != null) bits.push(`q${outcome.quality}`)
      if (outcome.downscaled) bits.push('downscaled')
      return { tone: 'good', label: saved > 0 ? `−${saved}%` : 'Compressed', detail: bits.join(' · ') }
    }
    case 'skippedUnderCap':
      return { tone: 'info', label: 'Under cap', detail: 'already below the target — copied as-is' }
    case 'skippedCollision':
      return { tone: 'warn', label: 'Skipped', detail: 'an output file already exists' }
    case 'unreachable':
      return { tone: 'bad', label: 'Unreachable', detail: outcome.reason }
    case 'failed':
      return { tone: 'bad', label: 'Failed', detail: outcome.reason }
    case 'cancelled':
      return { tone: 'neutral', label: 'Cancelled', detail: 'not processed' }
  }
}

/** Final size of a result, if it produced one. */
export function finalBytesOf(result: FileResult): number | null {
  switch (result.outcome.kind) {
    case 'compressed':
      return result.outcome.finalBytes
    case 'skippedUnderCap':
      return result.outcome.bytes
    default:
      return null
  }
}
