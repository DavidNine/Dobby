// F3 — Format Utils (pure formatting functions)
//
// Per HLD §4 F3 and §6:
//   - Network unit conversion (bytes/sec -> KB/s, MB/s, base 1024) lives ONLY here.
//   - Time is Unix epoch SECONDS (UTC) across all modules/API; timezone formatting
//     happens only in this frontend display layer.
//
// All functions are pure (no React, no I/O) and individually unit-testable.

const KIB = 1024
const BYTE_UNITS = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'] as const

/**
 * Humanize a byte count to B / KB / MB / GB ... using base 1024.
 * Bytes are shown as an integer; KB and larger get 1 decimal place.
 * 0 -> "0 B".
 */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B'

  let value = bytes
  let unitIndex = 0
  while (value >= KIB && unitIndex < BYTE_UNITS.length - 1) {
    value /= KIB
    unitIndex += 1
  }

  // Integer for raw bytes, 1 decimal for KB and above.
  const formatted = unitIndex === 0 ? String(Math.round(value)) : value.toFixed(1)
  return `${formatted} ${BYTE_UNITS[unitIndex]}`
}

const RATE_UNITS = ['B/s', 'KB/s', 'MB/s', 'GB/s', 'TB/s'] as const

/**
 * Format a transfer rate. Input is bytes/sec (the API's unit); auto-selects the
 * appropriate unit using base 1024. B/s shown as integer; KB/s and above get
 * 1 decimal place. 0 -> "0 B/s".
 */
export function formatRate(bps: number): string {
  if (!Number.isFinite(bps) || bps <= 0) return '0 B/s'

  let value = bps
  let unitIndex = 0
  while (value >= KIB && unitIndex < RATE_UNITS.length - 1) {
    value /= KIB
    unitIndex += 1
  }

  const formatted = unitIndex === 0 ? String(Math.round(value)) : value.toFixed(1)
  return `${formatted} ${RATE_UNITS[unitIndex]}`
}

/**
 * Format a percentage value with exactly 1 decimal place.
 * e.g. 23.456 -> "23.5%", 0 -> "0.0%", 100 -> "100.0%".
 */
export function formatPercent(value: number): string {
  const safe = Number.isFinite(value) ? value : 0
  return `${safe.toFixed(1)}%`
}

/** Zero-pad a number to 2 digits. */
function pad2(n: number): string {
  return String(n).padStart(2, '0')
}

/**
 * Convert a Unix epoch SECONDS timestamp to a local-time "HH:MM" display string.
 * The seconds value is multiplied by 1000 for the Date constructor.
 *
 * Testability note: this uses local-timezone Date getters (getHours/getMinutes),
 * so the output depends on the runtime timezone. Tests must derive the expected
 * string from the SAME Date API (new Date(ts*1000)) rather than hard-coding a
 * timezone-specific literal, which keeps tests robust in any TZ.
 */
export function formatTimestamp(ts: number): string {
  const d = new Date(ts * 1000)
  return `${pad2(d.getHours())}:${pad2(d.getMinutes())}`
}

/**
 * Axis label whose format depends on the selected time range.
 *   - Short ranges (1h / 6h / 24h) -> "HH:MM" (local time).
 *   - Long range (7d)              -> "MM/DD" (month/day, local date).
 *
 * Date-format choice (documented): the 7d label uses MM/DD (month-first), e.g.
 * June 3 -> "06/03". ts is Unix epoch SECONDS.
 *
 * Testability note (same as formatTimestamp): expected strings should be built
 * from new Date(ts*1000) getters in the test so assertions are TZ-robust.
 */
export function formatTimeAxis(ts: number, range: string): string {
  const d = new Date(ts * 1000)
  if (range === '7d') {
    // MM/DD — getMonth() is 0-based, so add 1.
    return `${pad2(d.getMonth() + 1)}/${pad2(d.getDate())}`
  }
  // 1h / 6h / 24h (and any other short range) -> HH:MM
  return `${pad2(d.getHours())}:${pad2(d.getMinutes())}`
}
