import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import MetricCard from './MetricCard'

describe('MetricCard', () => {
  it('renders the title and the formatted value', () => {
    render(<MetricCard title="CPU" value="23.5%" />)
    expect(screen.getByText('CPU')).toBeInTheDocument()
    expect(screen.getByText('23.5%')).toBeInTheDocument()
  })

  it('renders the optional subValue when provided', () => {
    render(<MetricCard title="RAM" value="8.0 GB" subValue="of 16.0 GB" />)
    expect(screen.getByText('of 16.0 GB')).toBeInTheDocument()
  })

  it('omits the subValue line when not provided', () => {
    render(<MetricCard title="CPU" value="23.5%" />)
    expect(screen.queryByText(/of /)).not.toBeInTheDocument()
  })
})
