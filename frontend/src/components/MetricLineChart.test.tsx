import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import type { HistoryPoint } from '../api/types'

// Mock react-chartjs-2 so tests don't need a real canvas. The mocked <Line>
// exposes the dataset's data points via a data-* attribute for assertions.
vi.mock('react-chartjs-2', () => ({
  Line: (props: { data?: { datasets?: Array<{ data?: unknown[] }> } }) => (
    <div
      data-testid="line-chart"
      data-points={JSON.stringify(props.data?.datasets?.[0]?.data ?? [])}
    />
  ),
}))

import MetricLineChart from './MetricLineChart'

const points: HistoryPoint[] = [
  { timestamp: 1_700_000_000, cpu_percent: 10, mem_percent: 40, net_rx_bps: 1000, net_tx_bps: 500 },
  { timestamp: 1_700_000_600, cpu_percent: 20, mem_percent: 45, net_rx_bps: 2000, net_tx_bps: 600 },
  { timestamp: 1_700_001_200, cpu_percent: 30, mem_percent: 50, net_rx_bps: 3000, net_tx_bps: 700 },
]

describe('MetricLineChart', () => {
  it('renders the (mocked) chart without crashing given a points array', () => {
    render(<MetricLineChart title="CPU" points={points} field="cpu_percent" range="1h" />)
    expect(screen.getByTestId('line-chart')).toBeInTheDocument()
  })

  it('passes the chosen field values as the chart data (one point per input)', () => {
    render(<MetricLineChart title="CPU" points={points} field="cpu_percent" range="1h" />)
    const data = JSON.parse(screen.getByTestId('line-chart').getAttribute('data-points')!)
    expect(data).toHaveLength(points.length)
    expect(data).toEqual([10, 20, 30])
  })

  it('renders with an empty points array', () => {
    render(<MetricLineChart title="CPU" points={[]} field="net_rx_bps" range="7d" />)
    const data = JSON.parse(screen.getByTestId('line-chart').getAttribute('data-points')!)
    expect(data).toEqual([])
  })
})
