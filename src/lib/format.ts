// Size formatting helpers. Drafted via local-model delegation, then corrected: the original
// applied `toFixed` to the bytes unit too (so 950 -> '950.0 B'), which broke the integer-bytes
// contract. Bytes are now whole numbers; KB and above use the requested precision.

export type SizeUnit = 'KB' | 'MB'

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'] as const

/** Human-readable size using base 1024. Bytes show as an integer; KB+ use `fractionDigits`. */
export function formatBytes(bytes: number, fractionDigits = 1): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '0 B'
  let value = bytes
  let unitIndex = 0
  while (value >= 1024 && unitIndex < UNITS.length - 1) {
    value /= 1024
    unitIndex += 1
  }
  const text = unitIndex === 0 ? String(Math.round(value)) : value.toFixed(fractionDigits)
  return `${text} ${UNITS[unitIndex]}`
}

/** Convert a `value` in `unit` to a non-negative integer number of bytes. */
export function parseSizeToBytes(value: number, unit: SizeUnit): number {
  if (!Number.isFinite(value) || value < 0) return 0
  const multiplier = unit === 'MB' ? 1024 * 1024 : 1024
  return Math.floor(value * multiplier)
}

/** Integer percent saved, clamped to 0..100. Returns 0 when there is no real saving. */
export function percentSaved(originalBytes: number, finalBytes: number): number {
  if (originalBytes <= 0 || finalBytes >= originalBytes) return 0
  const saved = ((originalBytes - finalBytes) / originalBytes) * 100
  return Math.round(Math.min(100, Math.max(0, saved)))
}
