import { render, screen, fireEvent } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import Settings from './Settings'

describe('Settings', () => {
  it('renders the five themes and marks the active one', () => {
    render(<Settings theme="dark" onChange={() => {}} />)

    expect(screen.getByRole('radio', { name: 'Default Dark' })).toHaveAttribute(
      'aria-checked',
      'true',
    )
    expect(screen.getByRole('radio', { name: 'Muji Minimalist' })).toHaveAttribute(
      'aria-checked',
      'false',
    )
    expect(screen.getByRole('radio', { name: 'Cyberpunk' })).toBeInTheDocument()
    expect(screen.getByRole('radio', { name: 'Dobby Cat' })).toBeInTheDocument()
    expect(screen.getByRole('radio', { name: 'Glassmorphism' })).toBeInTheDocument()
  })

  it('calls onChange with dobby-cat when the Dobby Cat swatch is clicked', () => {
    const onChange = vi.fn()
    render(<Settings theme="dark" onChange={onChange} />)

    fireEvent.click(screen.getByRole('radio', { name: 'Dobby Cat' }))
    expect(onChange).toHaveBeenCalledWith('dobby-cat')
  })

  it('calls onChange with the chosen theme id', () => {
    const onChange = vi.fn()
    render(<Settings theme="dark" onChange={onChange} />)

    fireEvent.click(screen.getByRole('radio', { name: 'Cyberpunk' }))
    expect(onChange).toHaveBeenCalledWith('cyberpunk')
  })
})
