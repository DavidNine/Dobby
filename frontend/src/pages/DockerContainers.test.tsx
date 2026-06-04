import { act, render, screen, fireEvent } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import DockerContainers from './DockerContainers'
import * as client from '../api/client'
import type { DockerContainer, DockerListResponse } from '../api/types'

vi.mock('../api/client', () => ({
  getContainers: vi.fn(),
  containerAction: vi.fn(),
}))

const getContainers = vi.mocked(client.getContainers)
const containerAction = vi.mocked(client.containerAction)

const container = (over: Partial<DockerContainer> = {}): DockerContainer => ({
  id: 'abc123',
  name: 'web',
  image: 'nginx:alpine',
  state: 'running',
  status: 'Up 3 minutes',
  created: 1000,
  ports: [],
  ...over,
})

const listOf = (containers: DockerContainer[]): DockerListResponse => ({
  available: true,
  error: null,
  containers,
})

beforeEach(() => {
  vi.useFakeTimers()
  Object.defineProperty(document, 'hidden', { configurable: true, get: () => false })
  getContainers.mockResolvedValue(listOf([container()]))
  containerAction.mockResolvedValue(undefined)
})

afterEach(() => {
  vi.clearAllMocks()
  vi.useRealTimers()
})

async function flush() {
  await act(async () => {
    await Promise.resolve()
    await Promise.resolve()
  })
}

describe('DockerContainers', () => {
  it('renders a row with name, image and status after fetch', async () => {
    render(<DockerContainers />)
    await flush()

    expect(screen.getByText('web')).toBeInTheDocument()
    expect(screen.getByText('nginx:alpine')).toBeInTheDocument()
    expect(screen.getByText('Up 3 minutes')).toBeInTheDocument()
    // Running containers expose Restart + Stop.
    expect(screen.getByRole('button', { name: 'Restart' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Stop' })).toBeInTheDocument()
  })

  it('shows only Start for a stopped container', async () => {
    getContainers.mockResolvedValue(listOf([container({ state: 'exited', status: 'Exited (0)' })]))
    render(<DockerContainers />)
    await flush()

    expect(screen.getByRole('button', { name: 'Start' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Stop' })).not.toBeInTheDocument()
  })

  it('calls the action client and refetches on Restart click', async () => {
    render(<DockerContainers />)
    await flush()
    const callsBefore = getContainers.mock.calls.length

    fireEvent.click(screen.getByRole('button', { name: 'Restart' }))
    await flush()

    expect(containerAction).toHaveBeenCalledWith('abc123', 'restart')
    // refresh() triggered an extra fetch.
    expect(getContainers.mock.calls.length).toBeGreaterThan(callsBefore)
  })

  it('surfaces a daemon-unavailable state', async () => {
    getContainers.mockResolvedValue({
      available: false,
      error: 'no socket',
      containers: [],
    })
    render(<DockerContainers />)
    await flush()

    expect(screen.getByRole('alert')).toHaveTextContent('Docker daemon unavailable: no socket')
  })

  it('shows an action error without crashing when the action rejects', async () => {
    containerAction.mockRejectedValue(new Error('boom'))
    render(<DockerContainers />)
    await flush()

    fireEvent.click(screen.getByRole('button', { name: 'Restart' }))
    await flush()

    expect(screen.getByRole('alert')).toHaveTextContent('restart web: boom')
  })
})
