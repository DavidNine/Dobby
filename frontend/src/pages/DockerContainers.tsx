// DockerContainers — view behind the "Docker Containers" sidebar tab.
//
// Renders a simple table (Name / Image / Status / Actions) of the host's
// containers, polled via useDockerContainers. Lifecycle buttons (Start / Stop /
// Restart) POST to the backend and refresh the list on completion; the row's
// buttons are disabled while one of its actions is in flight.

import { useState } from 'react'
import { containerAction } from '../api/client'
import type { ContainerAction, DockerContainer } from '../api/types'
import { useDockerContainers } from '../hooks/useDockerContainers'
import StatusLight from '../components/StatusLight'

/** Which lifecycle buttons to show for a given container state. */
function actionsFor(state: string): ContainerAction[] {
  return state === 'running' ? ['restart', 'stop'] : ['start']
}

const ACTION_LABEL: Record<ContainerAction, string> = {
  restart: 'Restart',
  start: 'Start',
  stop: 'Stop',
}

export default function DockerContainers() {
  const { data, loading, error, refresh } = useDockerContainers()
  // The container id with an action currently in flight (disables its buttons).
  const [pendingId, setPendingId] = useState<string | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)

  const runAction = async (c: DockerContainer, action: ContainerAction) => {
    setPendingId(c.id)
    setActionError(null)
    try {
      await containerAction(c.id, action)
      refresh()
    } catch (err) {
      setActionError(
        `${action} ${c.name}: ${err instanceof Error ? err.message : String(err)}`,
      )
    } finally {
      setPendingId(null)
    }
  }

  return (
    <div className="min-h-screen bg-slate-50 text-slate-800 dark:bg-gray-900 dark:text-gray-100">
      <div className="mx-auto max-w-6xl px-4 py-6 sm:px-6 lg:px-8">
        <header className="mb-6">
          <h1 className="text-2xl font-semibold">Docker Containers</h1>
          <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
            Containers on this host. Updates every few seconds.
          </p>
        </header>

        {actionError && (
          <div
            role="alert"
            className="mb-4 rounded-lg border border-red-200 bg-red-50 px-4 py-2 text-sm text-red-700 dark:border-red-900 dark:bg-red-950 dark:text-red-300"
          >
            {actionError}
          </div>
        )}

        <ContainersBody
          loading={loading}
          error={error}
          available={data?.available ?? true}
          daemonError={data?.error ?? null}
          containers={data?.containers ?? []}
          pendingId={pendingId}
          onAction={runAction}
        />
      </div>
    </div>
  )
}

interface BodyProps {
  loading: boolean
  error: Error | null
  available: boolean
  daemonError: string | null
  containers: DockerContainer[]
  pendingId: string | null
  onAction: (c: DockerContainer, action: ContainerAction) => void
}

function ContainersBody({
  loading,
  error,
  available,
  daemonError,
  containers,
  pendingId,
  onAction,
}: BodyProps) {
  if (loading) return <Notice>Loading containers…</Notice>
  if (error) return <Notice tone="error">Error: {error.message}</Notice>
  if (!available) {
    return (
      <Notice tone="error">
        Docker daemon unavailable{daemonError ? `: ${daemonError}` : ''}
      </Notice>
    )
  }
  if (containers.length === 0) return <Notice>No containers found.</Notice>

  return (
    <div className="overflow-x-auto rounded-xl border border-gray-200 bg-white shadow-sm dark:border-gray-700 dark:bg-gray-800">
      <table className="w-full text-left text-sm">
        <thead className="border-b border-gray-200 text-xs uppercase tracking-wide text-gray-500 dark:border-gray-700 dark:text-gray-400">
          <tr>
            <th className="px-4 py-3 font-medium">Name</th>
            <th className="px-4 py-3 font-medium">Image</th>
            <th className="px-4 py-3 font-medium">Status</th>
            <th className="px-4 py-3 text-right font-medium">Actions</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-gray-100 dark:divide-gray-700">
          {containers.map((c) => (
            <tr key={c.id}>
              <td className="px-4 py-3 font-mono font-medium">{c.name}</td>
              <td className="px-4 py-3 text-gray-600 dark:text-gray-300">{c.image}</td>
              <td className="px-4 py-3">
                <StatusLight state={c.state} status={c.status} />
              </td>
              <td className="px-4 py-3">
                <div className="flex justify-end gap-2">
                  {actionsFor(c.state).map((action) => (
                    <button
                      key={action}
                      type="button"
                      disabled={pendingId === c.id}
                      onClick={() => onAction(c, action)}
                      className="rounded-md border border-gray-300 px-2.5 py-1 text-xs font-medium text-gray-700 transition-colors hover:bg-gray-100 disabled:cursor-not-allowed disabled:opacity-50 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-700"
                    >
                      {pendingId === c.id ? '…' : ACTION_LABEL[action]}
                    </button>
                  ))}
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function Notice({
  children,
  tone = 'muted',
}: {
  children: React.ReactNode
  tone?: 'muted' | 'error'
}) {
  const isError = tone === 'error'
  const cls = isError
    ? 'border-red-200 bg-red-50 text-red-700 dark:border-red-900 dark:bg-red-950 dark:text-red-300'
    : 'border-gray-200 bg-white text-gray-500 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400'
  return (
    <div
      role={isError ? 'alert' : undefined}
      className={`rounded-xl border px-4 py-8 text-center text-sm shadow-sm ${cls}`}
    >
      {children}
    </div>
  )
}
