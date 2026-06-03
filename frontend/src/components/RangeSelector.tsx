// F5 — RangeSelector
//
// Pure presentational segmented control for the 1h / 6h / 24h / 7d time ranges.
// Highlights the active range and calls onChange(range) on click. No state.

import type { Range } from '../api/types'

export interface RangeSelectorProps {
  value: Range
  onChange: (r: Range) => void
}

const RANGES: Range[] = ['1h', '6h', '24h', '7d']

export default function RangeSelector({ value, onChange }: RangeSelectorProps) {
  return (
    <div role="group" aria-label="Time range" className="inline-flex rounded-lg border border-gray-200 bg-gray-50 p-1 dark:border-gray-700 dark:bg-gray-800">
      {RANGES.map((r) => {
        const active = r === value
        return (
          <button
            key={r}
            type="button"
            aria-pressed={active}
            data-active={active}
            onClick={() => onChange(r)}
            className={
              'rounded-md px-3 py-1 text-sm font-medium transition-colors ' +
              (active
                ? 'bg-blue-600 text-white shadow'
                : 'text-gray-600 hover:bg-gray-200 dark:text-gray-300 dark:hover:bg-gray-700')
            }
          >
            {r}
          </button>
        )
      })}
    </div>
  )
}
