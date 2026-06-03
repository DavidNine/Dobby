import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import StatusBar from './StatusBar'
import { formatTimestamp } from '../utils/format'

describe('StatusBar', () => {
  it('shows the formatted last-updated time when connected', () => {
    const ts = 1_700_000_000
    render(<StatusBar lastUpdated={ts} error={null} />)
    // Build the expected string from the same Date API to stay TZ-robust.
    expect(screen.getByText(`Last updated: ${formatTimestamp(ts)}`)).toBeInTheDocument()
  })

  it('shows the error message when an error is present', () => {
    render(<StatusBar lastUpdated={1_700_000_000} error={new Error('network down')} />)
    expect(screen.getByRole('alert')).toHaveTextContent('network down')
  })

  it('error takes priority over last-updated', () => {
    render(<StatusBar lastUpdated={1_700_000_000} error={new Error('boom')} />)
    expect(screen.queryByText(/Last updated/)).not.toBeInTheDocument()
  })

  it('shows a loading state when loading and no error', () => {
    render(<StatusBar lastUpdated={null} error={null} loading />)
    expect(screen.getByText(/Loading/)).toBeInTheDocument()
  })

  it('shows a waiting message when there is no data yet', () => {
    render(<StatusBar lastUpdated={null} error={null} />)
    expect(screen.getByText(/Waiting for data/)).toBeInTheDocument()
  })
})
