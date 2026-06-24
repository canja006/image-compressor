import { describe, it, expect } from 'vitest'
import { formatBytes, parseSizeToBytes, percentSaved } from './format'

describe('formatBytes', () => {
  it('returns "0 B" for invalid or negative input', () => {
    expect(formatBytes(-1)).toBe('0 B')
    expect(formatBytes(Number.POSITIVE_INFINITY)).toBe('0 B')
    expect(formatBytes(Number.NaN)).toBe('0 B')
  })

  it('shows whole bytes below 1 KB without decimals', () => {
    expect(formatBytes(0)).toBe('0 B')
    expect(formatBytes(950)).toBe('950 B')
    expect(formatBytes(1023)).toBe('1023 B')
  })

  it('formats KB/MB/GB at the unit boundaries', () => {
    expect(formatBytes(1024)).toBe('1.0 KB')
    expect(formatBytes(1536)).toBe('1.5 KB')
    expect(formatBytes(1048576)).toBe('1.0 MB')
    expect(formatBytes(5_500_000)).toBe('5.2 MB')
    expect(formatBytes(1024 ** 3)).toBe('1.0 GB')
  })

  it('honors a custom fraction-digit count for KB and above', () => {
    expect(formatBytes(1536, 0)).toBe('2 KB')
    expect(formatBytes(1536, 2)).toBe('1.50 KB')
  })
})

describe('parseSizeToBytes', () => {
  it('multiplies by the unit and floors to an integer', () => {
    expect(parseSizeToBytes(500, 'KB')).toBe(500 * 1024)
    expect(parseSizeToBytes(2, 'MB')).toBe(2 * 1024 * 1024)
    expect(parseSizeToBytes(1.5, 'KB')).toBe(Math.floor(1.5 * 1024))
  })

  it('returns 0 for negative or non-finite values', () => {
    expect(parseSizeToBytes(-5, 'KB')).toBe(0)
    expect(parseSizeToBytes(Number.NaN, 'MB')).toBe(0)
  })
})

describe('percentSaved', () => {
  it('computes the rounded integer percentage saved', () => {
    expect(percentSaved(1000, 500)).toBe(50)
    expect(percentSaved(1000, 250)).toBe(75)
    expect(percentSaved(3, 2)).toBe(33)
  })

  it('returns 0 when there is no real saving', () => {
    expect(percentSaved(1000, 1000)).toBe(0)
    expect(percentSaved(1000, 1200)).toBe(0)
    expect(percentSaved(0, 0)).toBe(0)
    expect(percentSaved(-10, 5)).toBe(0)
  })
})
