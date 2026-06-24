/** Convert an `[r, g, b]` triplet (0-255) to a `#rrggbb` hex string. */
export function rgbToHex([r, g, b]: [number, number, number]): string {
  const channel = (n: number) =>
    Math.max(0, Math.min(255, Math.round(n)))
      .toString(16)
      .padStart(2, '0')
  return `#${channel(r)}${channel(g)}${channel(b)}`
}

/** Parse a `#rrggbb` hex string to an `[r, g, b]` triplet, falling back to white. */
export function hexToRgb(hex: string): [number, number, number] {
  const match = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex)
  if (!match) return [255, 255, 255]
  return [parseInt(match[1], 16), parseInt(match[2], 16), parseInt(match[3], 16)]
}
