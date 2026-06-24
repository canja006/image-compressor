import type { InputFile } from './types'

/**
 * Distribute a total byte budget across files as per-file caps, proportional to each file's
 * original size. Because every output is `<= its cap` and the caps sum to (at most) the budget,
 * the combined output is guaranteed to fit the budget. A file whose share is too small to reach
 * is simply marked unreachable (and contributes nothing), leaving the budget under-used.
 */
export function splitBudget(inputs: InputFile[], budgetBytes: number): Record<string, number> {
  const result: Record<string, number> = {}
  if (inputs.length === 0 || budgetBytes <= 0) return result
  const totalOriginal = inputs.reduce((sum, f) => sum + Math.max(1, f.bytes), 0)
  for (const f of inputs) {
    result[f.path] = Math.max(1, Math.floor((budgetBytes * Math.max(1, f.bytes)) / totalOriginal))
  }
  return result
}
