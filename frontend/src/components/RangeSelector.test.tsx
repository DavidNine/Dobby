import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import RangeSelector from './RangeSelector'

describe('RangeSelector', () => {
  it('renders a button for every range', () => {
    render(<RangeSelector value="1h" onChange={() => {}} />)
    for (const r of ['1h', '6h', '24h', '7d']) {
      expect(screen.getByRole('button', { name: r })).toBeInTheDocument()
    }
  })

  it('calls onChange with the clicked range', async () => {
    const user = userEvent.setup()
    const onChange = vi.fn()
    render(<RangeSelector value="1h" onChange={onChange} />)
    await user.click(screen.getByRole('button', { name: '24h' }))
    expect(onChange).toHaveBeenCalledTimes(1)
    expect(onChange).toHaveBeenCalledWith('24h')
  })

  it('marks the active range', () => {
    render(<RangeSelector value="6h" onChange={() => {}} />)
    const active = screen.getByRole('button', { name: '6h' })
    expect(active).toHaveAttribute('aria-pressed', 'true')
    expect(active).toHaveAttribute('data-active', 'true')

    const inactive = screen.getByRole('button', { name: '1h' })
    expect(inactive).toHaveAttribute('aria-pressed', 'false')
    expect(inactive).toHaveAttribute('data-active', 'false')
  })
})
