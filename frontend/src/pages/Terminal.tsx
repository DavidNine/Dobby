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
import { useCssVar } from '../hooks/useCssVar'

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

/** xterm's canvas needs a solid color; the glass theme's --color-bg is a
 *  gradient, so fall back to a solid dark for the console in that case. */
function solidBg(value: string): string {
  return value.includes('gradient') ? '#0f172a' : value
}

export default function Terminal() {
  const hostRef = useRef<HTMLDivElement>(null)
  const termRef = useRef<XTerm | null>(null)

  // Follow the active theme (same approach as the charts): these re-read on
  // every `data-theme` change and drive the re-theme effect below.
  const bg = useCssVar('--color-bg', '#0f172a')
  const fg = useCssVar('--color-text-main', '#f1f5f9')
  const cursor = useCssVar('--color-accent', '#3b82f6')

  useEffect(() => {
    const host = hostRef.current
    if (!host) return

    // Read the initial colors straight from the DOM so the terminal boots with
    // the right palette — without taking the reactive values as effect deps,
    // which would recreate (and disconnect) the session on every theme switch.
    const css = getComputedStyle(document.documentElement)
    const initial = (name: string, fallback: string) =>
      css.getPropertyValue(name).trim() || fallback

    // 1. Boot the terminal + fit addon.
    const term = new XTerm({
      fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
      fontSize: 13,
      cursorBlink: true,
      theme: {
        background: solidBg(initial('--color-bg', '#0f172a')),
        foreground: initial('--color-text-main', '#f1f5f9'),
        cursor: initial('--color-accent', '#3b82f6'),
        cursorAccent: solidBg(initial('--color-bg', '#0f172a')),
      },
    })
    termRef.current = term
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
      termRef.current = null
    }
  }, [])

  // Re-theme the live terminal whenever the CSS theme variables change. Setting
  // `options.theme` makes xterm repaint immediately — the equivalent of the
  // charts' borderColor update + chart.update().
  useEffect(() => {
    const term = termRef.current
    if (!term) return
    term.options.theme = {
      background: solidBg(bg),
      foreground: fg,
      cursor,
      cursorAccent: solidBg(bg),
    }
  }, [bg, fg, cursor])

  return (
    <div className="flex h-full flex-col bg-background text-main">
      <div className="mx-auto flex w-full max-w-6xl flex-1 flex-col px-4 py-6 sm:px-6 lg:px-8">
        <header className="mb-4">
          <h1 className="text-2xl font-semibold">Terminal</h1>
          <p className="mt-1 text-sm text-muted">
            Live bash session over WebSocket.
          </p>
        </header>
        {/* Panel background follows --color-bg so the padding matches the
            terminal canvas, which is themed via xterm's options.theme. */}
        <div className="flex-1 overflow-hidden rounded-xl border border-border bg-background p-3 shadow-sm">
          <div ref={hostRef} className="h-full w-full" />
        </div>
      </div>
    </div>
  )
}
