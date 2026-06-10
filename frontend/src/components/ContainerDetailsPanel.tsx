// ContainerDetailsPanel — expanded-row content in the Docker Containers table.
//
// Fetches GET /api/docker/containers/{id} on mount and renders the inspect
// details: port bindings, mounts, networks, plus an overview (command, restart
// policy, timestamps) and collapsible environment/labels sections.

import { useEffect, useState } from 'react'
import { getContainerDetails } from '../api/client'
import type { DockerContainerDetails } from '../api/types'

const MUTED = 'text-gray-500 dark:text-gray-400'

/** Render an RFC 3339 timestamp in the viewer's locale ('—' when absent). */
function formatTime(iso: string | null): string {
  if (!iso) return '—'
  const date = new Date(iso)
  return Number.isNaN(date.getTime()) ? iso : date.toLocaleString()
}

export default function ContainerDetailsPanel({ id }: { id: string }) {
  const [details, setDetails] = useState<DockerContainerDetails | null>(null)
  const [error, setError] = useState<Error | null>(null)

  // No state reset on id change: the parent keys the panel by container id,
  // so a different id always mounts a fresh instance.
  useEffect(() => {
    let cancelled = false
    getContainerDetails(id)
      .then((d) => {
        if (!cancelled) setDetails(d)
      })
      .catch((err) => {
        if (!cancelled)
          setError(err instanceof Error ? err : new Error(String(err)))
      })
    return () => {
      cancelled = true
    }
  }, [id])

  if (error) {
    return (
      <p role="alert" className="text-sm text-red-600 dark:text-red-400">
        Failed to load details: {error.message}
      </p>
    )
  }
  if (!details) {
    return <p className={`text-sm ${MUTED}`}>Loading details…</p>
  }

  return (
    <div className="grid gap-4 text-sm lg:grid-cols-2">
      <Section title="Overview">
        <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
          <Field label="ID" mono>
            {details.id}
          </Field>
          <Field label="Image" mono>
            {details.image}
          </Field>
          <Field label="Command" mono>
            {details.command ?? '—'}
          </Field>
          {details.working_dir && (
            <Field label="Working dir" mono>
              {details.working_dir}
            </Field>
          )}
          <Field label="Restart policy">
            {details.restart_policy ?? '—'}
            {details.restart_count > 0 &&
              ` (restarted ${details.restart_count}×)`}
          </Field>
          {details.platform && <Field label="Platform">{details.platform}</Field>}
          <Field label="Created">{formatTime(details.created)}</Field>
          <Field label="Started">{formatTime(details.started_at)}</Field>
          {details.finished_at && (
            <Field label="Finished">
              {formatTime(details.finished_at)}
              {details.exit_code !== null && ` (exit code ${details.exit_code})`}
            </Field>
          )}
        </dl>
      </Section>

      <Section title="Network ports">
        {details.ports.length === 0 ? (
          <Empty>No exposed ports.</Empty>
        ) : (
          <DetailTable head={['Container port', 'Host binding']}>
            {details.ports.map((p, i) => (
              <tr key={i}>
                <td className="py-1 pr-4 font-mono">
                  {p.container_port}/{p.protocol}
                </td>
                <td className="py-1 font-mono">
                  {p.host_port !== null
                    ? `${p.host_ip ?? '0.0.0.0'}:${p.host_port}`
                    : 'not published'}
                </td>
              </tr>
            ))}
          </DetailTable>
        )}
      </Section>

      <Section title="Mount points">
        {details.mounts.length === 0 ? (
          <Empty>No mounts.</Empty>
        ) : (
          <DetailTable head={['Type', 'Source', 'Destination', 'Mode']}>
            {details.mounts.map((m, i) => (
              <tr key={i}>
                <td className="py-1 pr-4">{m.type ?? '—'}</td>
                <td className="break-all py-1 pr-4 font-mono">
                  {m.source ?? m.name ?? '—'}
                </td>
                <td className="break-all py-1 pr-4 font-mono">
                  {m.destination ?? '—'}
                </td>
                <td className="py-1">{m.rw === false ? 'ro' : m.mode || 'rw'}</td>
              </tr>
            ))}
          </DetailTable>
        )}
      </Section>

      <Section title="Networks">
        {details.networks.length === 0 ? (
          <Empty>No networks.</Empty>
        ) : (
          <DetailTable head={['Network', 'IP address', 'Gateway', 'MAC']}>
            {details.networks.map((n) => (
              <tr key={n.name}>
                <td className="py-1 pr-4">{n.name}</td>
                <td className="py-1 pr-4 font-mono">{n.ip_address ?? '—'}</td>
                <td className="py-1 pr-4 font-mono">{n.gateway ?? '—'}</td>
                <td className="py-1 font-mono">{n.mac_address ?? '—'}</td>
              </tr>
            ))}
          </DetailTable>
        )}
      </Section>

      {(details.env.length > 0 || Object.keys(details.labels).length > 0) && (
        <div className="lg:col-span-2">
          {details.env.length > 0 && (
            <Collapsible summary={`Environment (${details.env.length})`}>
              {details.env.map((line, i) => (
                <div key={i} className="break-all">
                  {line}
                </div>
              ))}
            </Collapsible>
          )}
          {Object.keys(details.labels).length > 0 && (
            <Collapsible summary={`Labels (${Object.keys(details.labels).length})`}>
              {Object.entries(details.labels).map(([k, v]) => (
                <div key={k} className="break-all">
                  {k}={v}
                </div>
              ))}
            </Collapsible>
          )}
        </div>
      )}
    </div>
  )
}

function Section({
  title,
  children,
}: {
  title: string
  children: React.ReactNode
}) {
  return (
    <section>
      <h3 className={`mb-2 text-xs font-medium uppercase tracking-wide ${MUTED}`}>
        {title}
      </h3>
      {children}
    </section>
  )
}

function Field({
  label,
  mono = false,
  children,
}: {
  label: string
  mono?: boolean
  children: React.ReactNode
}) {
  return (
    <>
      <dt className={MUTED}>{label}</dt>
      <dd className={mono ? 'break-all font-mono' : ''}>{children}</dd>
    </>
  )
}

function DetailTable({
  head,
  children,
}: {
  head: string[]
  children: React.ReactNode
}) {
  return (
    <table className="w-full text-left">
      <thead className={`text-xs ${MUTED}`}>
        <tr>
          {head.map((h) => (
            <th key={h} className="py-1 pr-4 font-medium">
              {h}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>{children}</tbody>
    </table>
  )
}

function Collapsible({
  summary,
  children,
}: {
  summary: string
  children: React.ReactNode
}) {
  return (
    <details className="mt-1">
      <summary
        className={`cursor-pointer select-none ${MUTED} hover:text-gray-700 dark:hover:text-gray-200`}
      >
        {summary}
      </summary>
      <div className="mt-1 rounded-md bg-gray-50 px-3 py-2 font-mono text-xs dark:bg-gray-900">
        {children}
      </div>
    </details>
  )
}

function Empty({ children }: { children: React.ReactNode }) {
  return <p className={MUTED}>{children}</p>
}
