// F6 — Dashboard container (HLD §4 F6, §5(C)).
//
// Composes the polling hook (F4) with the presentational components (F5):
//   - manages the selected time `range` and feeds it to useMetricsPolling,
//   - renders the live metric cards (CPU / RAM / Network), formatting raw
//     numbers from `current` via the F3 format utils,
//   - renders the history line charts from `history.points`,
//   - renders the RangeSelector (changes range) and StatusBar (status line).
//
// Card layout decision: the network rates are shown as a SINGLE card with the
// download rate as the big value and the upload rate as the subValue. This
// keeps the top row to three cards (CPU / RAM / Network) and a clean
// 3-column grid on desktop / single column on mobile.

import { useState } from 'react'
import { useMetricsPolling } from '../hooks/usePolling'
import type { Range } from '../api/types'
import MetricCard from '../components/MetricCard'
import MetricLineChart from '../components/MetricLineChart'
import RangeSelector from '../components/RangeSelector'
import StatusBar from '../components/StatusBar'
import { formatBytes, formatPercent, formatRate } from '../utils/format'

const PLACEHOLDER = '—'

export default function Dashboard() {
  const [range, setRange] = useState<Range>('1h')
  const { current, history, loading, error, lastUpdated } = useMetricsPolling(range)

  const points = history?.points ?? []

  // Pre-format the live values (or fall back to a placeholder when no data yet).
  const cpuValue = current ? formatPercent(current.cpu.percent) : PLACEHOLDER

  const ramValue = current ? formatPercent(current.memory.percent) : PLACEHOLDER
  const ramSub = current
    ? `${formatBytes(current.memory.used_bytes)} / ${formatBytes(current.memory.total_bytes)}`
    : undefined

  const netDownValue = current ? formatRate(current.network.rx_bps) : PLACEHOLDER
  const netUpSub = current ? `↑ ${formatRate(current.network.tx_bps)}` : undefined

  return (
    <div className="min-h-screen bg-background text-main">
      <div className="mx-auto max-w-6xl px-4 py-6 sm:px-6 lg:px-8">
        <header className="mb-6 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl font-semibold">Dobby — System Monitor</h1>
            <div className="mt-1">
              <StatusBar lastUpdated={lastUpdated} error={error} loading={loading} />
            </div>
          </div>
          <RangeSelector value={range} onChange={setRange} />
        </header>

        {/* Live metric cards: single column on mobile, three columns on desktop. */}
        <section
          aria-label="Current metrics"
          className="mb-6 grid grid-cols-1 gap-4 md:grid-cols-3"
        >
          <MetricCard title="CPU" value={cpuValue} />
          <MetricCard title="RAM" value={ramValue} subValue={ramSub} />
          <MetricCard title="Network ↓" value={netDownValue} subValue={netUpSub} />
        </section>

        {/* History charts: stacked on mobile, two columns on large screens. */}
        <section
          aria-label="Metric history"
          className="grid grid-cols-1 gap-4 lg:grid-cols-2"
        >
          <MetricLineChart title="CPU %" points={points} field="cpu_percent" range={range} />
          <MetricLineChart title="RAM %" points={points} field="mem_percent" range={range} />
          <MetricLineChart
            title="Network Download (rx)"
            points={points}
            field="net_rx_bps"
            range={range}
          />
          <MetricLineChart
            title="Network Upload (tx)"
            points={points}
            field="net_tx_bps"
            range={range}
          />
        </section>
      </div>
    </div>
  )
}
