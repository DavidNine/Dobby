// Terminal — interactive shell behind the "Terminal" sidebar tab.
//
// On mount (i.e. when the user switches to this tab) it boots an xterm.js
// terminal and opens a WebSocket to the backend's `/ws/terminal`, which runs a
// real PTY-backed bash. The wiring is intentionally thin:
//   - keystrokes (xterm `onData`) → WS **binary** frames → PTY stdin,
//   - PTY output → WS **binary** frames → `term.write`,
//   - container/window resizes → WS **text(JSON)** `{type:"resize",cols,rows}`.
// Everything is torn down on unmount so switching away closes the socket and
// the backend reaps bash.

import { useEffect, useRef } from 'react'
import { Terminal as XTerm } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'

/**
 * Resolve the `/ws/terminal` URL. Mirrors the HTTP client's `VITE_API_BASE`
 * convention (so dev points at the backend port), converting the http(s)
 * scheme to ws(s); falls back to the current origin when unset.
 */
function terminalWsUrl(): string {
  const base = import.meta.env.VITE_API_BASE ?? ''
  if (base) {
    return base.replace(/^http/, 'ws') + '/ws/terminal'
  }
  const proto = window.location.protocol === 'https:' ? 'wss' : 'ws'
  return `${proto}://${window.location.host}/ws/terminal`
}

export default function Terminal() {
  const hostRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const host = hostRef.current
    if (!host) return

    // 1. Boot the terminal + fit addon.
    const term = new XTerm({
      fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
      fontSize: 13,
      cursorBlink: true,
      theme: { background: '#0f172a' }, // slate-900, matches the panel chrome
    })
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(host)
    fit.fit()

    // 2. Connect to the backend PTY shell.
    const ws = new WebSocket(terminalWsUrl())
    ws.binaryType = 'arraybuffer'

    // Refit locally and tell the PTY the new geometry.
    const syncSize = () => {
      fit.fit()
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: 'resize', cols: term.cols, rows: term.rows }))
      }
    }

    ws.onopen = () => {
      term.focus()
      syncSize()
    }
    // Server → terminal: raw PTY output (binary) or any stray text.
    ws.onmessage = (ev) => {
      term.write(
        ev.data instanceof ArrayBuffer ? new Uint8Array(ev.data) : (ev.data as string),
      )
    }
    ws.onclose = () => term.write('\r\n\x1b[90m[session closed]\x1b[0m\r\n')

    // 3. Terminal → server: keystrokes as binary frames.
    const dataSub = term.onData((data) => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(new TextEncoder().encode(data))
      }
    })

    // 4. Keep the PTY size in sync with the container (and the window).
    window.addEventListener('resize', syncSize)
    const ro = new ResizeObserver(syncSize)
    ro.observe(host)

    return () => {
      window.removeEventListener('resize', syncSize)
      ro.disconnect()
      dataSub.dispose()
      ws.close()
      term.dispose()
    }
  }, [])

  return (
    <div className="flex h-full flex-col bg-slate-50 text-slate-800 dark:bg-gray-900 dark:text-gray-100">
      <div className="mx-auto flex w-full max-w-6xl flex-1 flex-col px-4 py-6 sm:px-6 lg:px-8">
        <header className="mb-4">
          <h1 className="text-2xl font-semibold">Terminal</h1>
          <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
            Live bash session over WebSocket.
          </p>
        </header>
        <div className="flex-1 overflow-hidden rounded-xl border border-gray-200 bg-[#0f172a] p-3 shadow-sm dark:border-gray-700">
          <div ref={hostRef} className="h-full w-full" />
        </div>
      </div>
    </div>
  )
}
