// F5 — StatusBar
//
// Pure presentational status line: shows loading / error / last-updated state.
// Priority: error (if set) > loading (if true) > connected (with last-updated).
// lastUpdated is a Unix epoch SECONDS timestamp (formatted via F3 formatTimestamp).

import { formatTimestamp } from '../utils/format'

export interface StatusBarProps {
  lastUpdated: number | null
  error: Error | null
  loading?: boolean
}

export default function StatusBar({ lastUpdated, error, loading }: StatusBarProps) {
  if (error) {
    return (
      <div role="alert" className="flex items-center gap-2 text-sm text-red-600 dark:text-red-400">
        <span aria-hidden className="inline-block h-2 w-2 rounded-full bg-red-500" />
        <span>Error: {error.message}</span>
      </div>
    )
  }

  if (loading) {
    return (
      <div role="status" className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
        <span aria-hidden className="inline-block h-2 w-2 rounded-full bg-yellow-400" />
        <span>Loading…</span>
      </div>
    )
  }

  return (
    <div role="status" className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
      <span aria-hidden className="inline-block h-2 w-2 rounded-full bg-green-500" />
      <span>
        {lastUpdated !== null
          ? `Last updated: ${formatTimestamp(lastUpdated)}`
          : 'Waiting for data…'}
      </span>
    </div>
  )
}
