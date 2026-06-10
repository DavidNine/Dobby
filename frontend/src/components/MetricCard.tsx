// F5 — MetricCard
//
// Pure presentational big-number card. Displays an ALREADY-FORMATTED string
// `value` (the F6 Dashboard is responsible for formatting raw numbers via the
// F3 format utils, e.g. formatPercent / formatRate, before passing them here).
// This keeps the card free of unit/formatting logic and trivially testable.
//
// Props:
//   - title:    label shown above the big number (e.g. "CPU").
//   - value:    preformatted big-number string (e.g. "23.5%", "1.2 MB/s").
//   - subValue: optional secondary line below the number (e.g. "of 16.0 GB").

export interface MetricCardProps {
  title: string
  value: string
  subValue?: string
}

export default function MetricCard({ title, value, subValue }: MetricCardProps) {
  return (
    <div className="glass-surface rounded-xl border border-border bg-card p-4 shadow-sm">
      <div className="text-sm font-medium uppercase tracking-wide text-muted">
        {title}
      </div>
      <div className="mt-2 text-3xl font-bold leading-tight text-main sm:text-4xl">
        {value}
      </div>
      {subValue !== undefined && (
        <div className="mt-1 text-sm text-muted">{subValue}</div>
      )}
    </div>
  )
}
