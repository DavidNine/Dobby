import { act, render, screen, fireEvent } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Dashboard from './Dashboard'
import * as client from '../api/client'
import type { CurrentResponse, HistoryResponse } from '../api/types'

// Mock the API client so the polling hook resolves with known data.
vi.mock('../api/client', () => ({
  getCurrent: vi.fn(),
  getHistory: vi.fn(),
}))

// Mock react-chartjs-2 to avoid pulling in canvas under jsdom.
vi.mock('react-chartjs-2', () => ({
  Line: () => <div data-testid="line-chart" />,
}))

const getCurrent = vi.mocked(client.getCurrent)
const getHistory = vi.mocked(client.getHistory)

const sampleCurrent: CurrentResponse = {
  timestamp: 1000,
  cpu: { percent: 23.456 },
  memory: { total_bytes: 16 * 1024 * 1024 * 1024, used_bytes: 8 * 1024 * 1024 * 1024, percent: 50 },
  network: { rx_bps: 2 * 1024 * 1024, tx_bps: 512 * 1024 },
}

const makeHistory = (range: string): HistoryResponse => ({
  range,
  bucket_secs: 60,
  points: [
    { timestamp: 1000, cpu_percent: 10, mem_percent: 40, net_rx_bps: 100, net_tx_bps: 50 },
    { timestamp: 1060, cpu_percent: 20, mem_percent: 45, net_rx_bps: 200, net_tx_bps: 60 },
  ],
})

beforeEach(() => {
  vi.useFakeTimers()
  getCurrent.mockResolvedValue(sampleCurrent)
  getHistory.mockImplementation(async (range) => makeHistory(range))
  // Ensure the page is considered visible so polling starts.
  Object.defineProperty(document, 'hidden', { configurable: true, get: () => false })
})

afterEach(() => {
  vi.clearAllMocks()
  vi.useRealTimers()
})

/** Flush pending microtasks/promises under fake timers. */
async function flush() {
  await act(async () => {
    await Promise.resolve()
    await Promise.resolve()
  })
}

describe('Dashboard', () => {
  it('renders formatted CPU / RAM / network values and charts after the initial fetch', async () => {
    render(<Dashboard />)
    await flush()

    // Hook resolved.
    expect(getCurrent).toHaveBeenCalledTimes(1)
    expect(getHistory).toHaveBeenCalledWith('1h')

    // CPU formatted via formatPercent.
    expect(screen.getByText('23.5%')).toBeInTheDocument()
    // RAM percent + used/total subValue.
    expect(screen.getByText('50.0%')).toBeInTheDocument()
    expect(screen.getByText('8.0 GB / 16.0 GB')).toBeInTheDocument()
    // Network download (rx) as big value, upload (tx) as subValue.
    expect(screen.getByText('2.0 MB/s')).toBeInTheDocument()
    expect(screen.getByText('↑ 512.0 KB/s')).toBeInTheDocument()

    // Four charts rendered (CPU / RAM / rx / tx).
    expect(screen.getAllByTestId('line-chart')).toHaveLength(4)
  })

  it('shows placeholders and does not crash when current is null', async () => {
    getCurrent.mockResolvedValue(null)
    render(<Dashboard />)
    await flush()

    // Three cards each show the placeholder.
    expect(screen.getAllByText('—')).toHaveLength(3)
    // Charts still render with the (empty-from-current) history.
    expect(screen.getAllByTestId('line-chart')).toHaveLength(4)
  })

  it('refetches history with the new range when a range button is clicked', async () => {
    render(<Dashboard />)
    await flush()
    expect(getHistory).toHaveBeenCalledWith('1h')
    const callsBefore = getHistory.mock.calls.length

    // Click the "24h" range button.
    fireEvent.click(screen.getByRole('button', { name: '24h' }))
    await flush()

    expect(getHistory.mock.calls.length).toBeGreaterThan(callsBefore)
    expect(getHistory).toHaveBeenLastCalledWith('24h')
  })

  it('surfaces an error in the StatusBar without crashing when the client rejects', async () => {
    getCurrent.mockRejectedValue(new Error('network down'))
    getHistory.mockRejectedValue(new Error('network down'))
    render(<Dashboard />)
    await flush()

    const alert = screen.getByRole('alert')
    expect(alert).toHaveTextContent('Error: network down')
    // Cards still render placeholders (no crash).
    expect(screen.getAllByText('—')).toHaveLength(3)
  })
})
