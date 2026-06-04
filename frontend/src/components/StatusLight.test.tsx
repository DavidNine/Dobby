import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import StatusLight, { stateColor } from './StatusLight'

describe('stateColor', () => {
  it('maps running to green', () => {
    expect(stateColor('running')).toBe('green')
  })

  it('maps transitional states to amber', () => {
    expect(stateColor('restarting')).toBe('amber')
    expect(stateColor('paused')).toBe('amber')
    expect(stateColor('stopping')).toBe('amber')
  })

  it('maps dead to red', () => {
    expect(stateColor('dead')).toBe('red')
  })

  it('maps stopped/unknown states to slate', () => {
    expect(stateColor('exited')).toBe('slate')
    expect(stateColor('created')).toBe('slate')
    expect(stateColor('removing')).toBe('slate')
    expect(stateColor('whatever')).toBe('slate')
  })
})

describe('StatusLight', () => {
  it('renders the state label and optional status detail', () => {
    render(<StatusLight state="running" status="Up 2 hours" />)
    expect(screen.getByText('running')).toBeInTheDocument()
    expect(screen.getByText('Up 2 hours')).toBeInTheDocument()
  })

  it('renders without a status detail', () => {
    render(<StatusLight state="exited" />)
    expect(screen.getByText('exited')).toBeInTheDocument()
  })
})
