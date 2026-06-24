import { describe, it, expect } from 'vitest'
import { hexToRgb, rgbToHex } from './color'

describe('rgbToHex', () => {
  it('formats and clamps channels', () => {
    expect(rgbToHex([255, 255, 255])).toBe('#ffffff')
    expect(rgbToHex([0, 0, 0])).toBe('#000000')
    expect(rgbToHex([247, 246, 243])).toBe('#f7f6f3')
    expect(rgbToHex([300, -5, 128])).toBe('#ff0080')
  })
})

describe('hexToRgb', () => {
  it('parses with or without a leading hash and is case-insensitive', () => {
    expect(hexToRgb('#ffffff')).toEqual([255, 255, 255])
    expect(hexToRgb('000000')).toEqual([0, 0, 0])
    expect(hexToRgb('#F7F6F3')).toEqual([247, 246, 243])
  })

  it('falls back to white on invalid input', () => {
    expect(hexToRgb('nope')).toEqual([255, 255, 255])
    expect(hexToRgb('#fff')).toEqual([255, 255, 255])
  })
})

describe('round-trip', () => {
  it('survives rgb -> hex -> rgb', () => {
    expect(hexToRgb(rgbToHex([12, 34, 56]))).toEqual([12, 34, 56])
  })
})
