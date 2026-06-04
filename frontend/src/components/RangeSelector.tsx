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
    <div role="group" aria-label="Time range" className="inline-flex rounded-lg border border-border bg-card p-1">
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
                ? 'bg-accent text-accent-contrast shadow'
                : 'text-muted hover:bg-background')
            }
          >
            {r}
          </button>
        )
      })}
    </div>
  )
}
