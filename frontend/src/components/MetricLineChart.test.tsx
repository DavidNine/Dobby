import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import type { HistoryPoint } from '../api/types'

// Mock react-chartjs-2 so tests don't need a real canvas. The mocked <Line>
// exposes the dataset's data points and the chart options (scales/titles) via
// data-* attributes for assertions.
vi.mock('react-chartjs-2', () => ({
  Line: (props: {
    data?: { datasets?: Array<{ data?: unknown[] }> }
    options?: unknown
  }) => (
    <div
      data-testid="line-chart"
      data-points={JSON.stringify(props.data?.datasets?.[0]?.data ?? [])}
      data-options={JSON.stringify(props.options ?? {})}
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

  // Helper: read back the options object the component handed to <Line>.
  const readOptions = () =>
    JSON.parse(screen.getByTestId('line-chart').getAttribute('data-options')!)

  it('labels the x-axis as "Time"', () => {
    render(<MetricLineChart title="CPU" points={points} field="cpu_percent" range="1h" />)
    const options = readOptions()
    expect(options.scales.x.title).toEqual({ display: true, text: 'Time' })
  })

  it('labels the y-axis with the percent unit for percent fields', () => {
    render(<MetricLineChart title="RAM %" points={points} field="mem_percent" range="1h" />)
    const options = readOptions()
    expect(options.scales.y.title).toEqual({ display: true, text: 'Usage (%)' })
  })

  it('labels the y-axis with the bytes/sec unit for network fields', () => {
    render(<MetricLineChart title="Download" points={points} field="net_rx_bps" range="1h" />)
    const options = readOptions()
    expect(options.scales.y.title).toEqual({ display: true, text: 'Rate (Bytes/sec)' })
  })

  it('lets a caller override the y-axis label via the yLabel prop', () => {
    render(
      <MetricLineChart
        title="CPU"
        points={points}
        field="cpu_percent"
        range="1h"
        yLabel="CPU load (%)"
      />,
    )
    const options = readOptions()
    expect(options.scales.y.title).toEqual({ display: true, text: 'CPU load (%)' })
  })
})
