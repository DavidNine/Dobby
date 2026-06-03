import { describe, expect, it } from 'vitest'
import {
  formatBytes,
  formatRate,
  formatPercent,
  formatTimestamp,
  formatTimeAxis,
} from './format'

// TZ-robust time testing: the time/date functions use local Date getters, so the
// output depends on the runtime timezone. Instead of hard-coding a timezone-
// specific literal, we derive the expected string from the SAME Date API the
// implementation uses (new Date(ts*1000)). These helpers mirror format.ts so the
// tests pass in any timezone.
const pad2 = (n: number) => String(n).padStart(2, '0')
const expectedHHMM = (ts: number) => {
  const d = new Date(ts * 1000)
  return `${pad2(d.getHours())}:${pad2(d.getMinutes())}`
}
const expectedMMDD = (ts: number) => {
  const d = new Date(ts * 1000)
  return `${pad2(d.getMonth() + 1)}/${pad2(d.getDate())}`
}

describe('formatBytes', () => {
  it('formats 0 as "0 B"', () => {
    expect(formatBytes(0)).toBe('0 B')
  })

  it('formats 1023 as bytes (just under 1 KB)', () => {
    expect(formatBytes(1023)).toBe('1023 B')
  })

  it('formats 1024 as "1.0 KB" (KB boundary)', () => {
    expect(formatBytes(1024)).toBe('1.0 KB')
  })

  it('formats an MB-scale value', () => {
    // 1.5 MB = 1.5 * 1024 * 1024
    expect(formatBytes(1.5 * 1024 * 1024)).toBe('1.5 MB')
  })

  it('formats a GB-scale boundary', () => {
    expect(formatBytes(1024 * 1024 * 1024)).toBe('1.0 GB')
  })
})

describe('formatRate', () => {
  it('formats 0 as "0 B/s"', () => {
    expect(formatRate(0)).toBe('0 B/s')
  })

  it('formats a low rate in B/s', () => {
    expect(formatRate(512)).toBe('512 B/s')
  })

  it('switches to KB/s for a mid rate', () => {
    expect(formatRate(125000)).toBe('122.1 KB/s')
  })

  it('switches to MB/s for a high rate', () => {
    // 5 MB/s
    expect(formatRate(5 * 1024 * 1024)).toBe('5.0 MB/s')
  })
})

describe('formatPercent', () => {
  it('rounds to 1 decimal place', () => {
    expect(formatPercent(23.456)).toBe('23.5%')
  })

  it('formats boundary 0 as "0.0%"', () => {
    expect(formatPercent(0)).toBe('0.0%')
  })

  it('formats boundary 100 as "100.0%"', () => {
    expect(formatPercent(100)).toBe('100.0%')
  })
})

describe('formatTimestamp', () => {
  it('formats a fixed epoch-seconds input as local HH:MM', () => {
    const ts = 1717400000
    expect(formatTimestamp(ts)).toBe(expectedHHMM(ts))
  })

  it('zero-pads hours and minutes (HH:MM shape)', () => {
    const ts = 1717400000
    expect(formatTimestamp(ts)).toMatch(/^\d{2}:\d{2}$/)
  })
})

describe('formatTimeAxis', () => {
  const ts = 1717400000

  it('uses HH:MM time format for short range 1h', () => {
    expect(formatTimeAxis(ts, '1h')).toBe(expectedHHMM(ts))
  })

  it('uses HH:MM time format for short range 6h', () => {
    expect(formatTimeAxis(ts, '6h')).toBe(expectedHHMM(ts))
  })

  it('uses HH:MM time format for short range 24h', () => {
    expect(formatTimeAxis(ts, '24h')).toBe(expectedHHMM(ts))
  })

  it('uses MM/DD date format for long range 7d', () => {
    expect(formatTimeAxis(ts, '7d')).toBe(expectedMMDD(ts))
    expect(formatTimeAxis(ts, '7d')).toMatch(/^\d{2}\/\d{2}$/)
  })
})
