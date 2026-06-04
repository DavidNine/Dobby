//! B8 — WebSocket terminal.
//!
//! Bridges a browser xterm.js session to a real interactive `bash` shell
//! running behind a pseudo-terminal (PTY). Exposed as `GET /ws/terminal`.
//!
//! Wire protocol (kept deliberately small and unambiguous):
//!   - client → server **Binary**: raw keystroke bytes, written to PTY stdin.
//!   - client → server **Text(JSON)**: a control frame, currently only
//!       `{"type":"resize","cols":<u16>,"rows":<u16>}` (xterm fit-addon).
//!   - server → client **Binary**: raw PTY output bytes (xterm writes them).
//!
//! Concurrency model: portable-pty's reader/writer are *blocking* I/O, so the
//! two blocking halves live on dedicated OS threads and talk to the async side
//! over mpsc channels. Two async tasks then pump bytes between those channels
//! and the split WebSocket. When either direction ends we abort the other and
//! reap the shell so no zombie `bash` is left behind.

use std::io::{Read, Write};
use std::path::Path;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Deserialize;
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Security / threat model
// ---------------------------------------------------------------------------
//
// This endpoint is, by design, *arbitrary command execution*: a real shell.
// There is no "eval injection" to patch — running commands IS the feature — so
// security cannot come from filtering the byte stream. It comes from two
// layers, in order of importance:
//
//   1. ACCESS CONTROL (deployment, NOT this file): who can reach the socket.
//      Bind to localhost / put it behind authentication, and run the *service*
//      as a dedicated non-root user so a session is never a root shell. An
//      unauthenticated `0.0.0.0` root shell is the real vulnerability.
//
//   2. PROCESS CONFINEMENT (this file, defense in depth): the shell is pinned
//      to an absolute, validated `/bin/bash` (no `$PATH` hijack), spawned with
//      a *scrubbed* environment (so backend secrets / inherited vars never leak
//      into the session) and a fixed working directory.
//
// The process inherits the privileges of the backend process: confine those at
// the unit level (`User=`, `NoNewPrivileges=`, `ProtectSystem=`, …).

/// Absolute, fixed path to the only program this endpoint will ever launch.
/// Pinned (rather than relying on `$PATH`) so the shell cannot be redirected to
/// an attacker-planted `bash` earlier on the search path.
const SHELL_PATH: &str = "/bin/bash";

/// Minimal `$PATH` handed to the shell; avoids inheriting an attacker-influenced
/// path from the backend's own environment.
const SHELL_ENV_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

/// Initial PTY geometry; the client immediately re-sizes via a control frame
/// once xterm's fit-addon has measured the container.
const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;

/// Size of the PTY read buffer (one read = at most this many bytes forwarded).
const READ_BUF: usize = 8192;

/// Control frames sent by the client as JSON text frames.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ControlMsg {
    /// Terminal was resized to `cols` × `rows` character cells.
    Resize { cols: u16, rows: u16 },
}

/// A spawned shell and the PTY master used to talk to it.
struct Shell {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

/// `GET /ws/terminal` — upgrade the HTTP connection to a WebSocket and hand the
/// socket to [`handle_socket`].
pub async fn terminal_ws(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

/// Open a PTY and spawn an interactive `/bin/bash` inside it.
///
/// Hardening (see the module-level "Security / threat model" note):
///   - the program is pinned to the absolute [`SHELL_PATH`] and its existence
///     verified, so the endpoint can *only* launch `/bin/bash` — never some
///     other binary picked up via `$PATH`,
///   - the child's environment is cleared and rebuilt from a minimal allow-list
///     so the backend's own environment (any secrets/tokens) never leaks in,
///   - the working directory is fixed to `$HOME` (or `/` as a fallback).
fn spawn_shell() -> Result<Shell, Box<dyn std::error::Error>> {
    // Refuse to start unless the pinned shell actually exists. `is_file`
    // follows the `/bin/bash → /usr/bin/bash` symlink.
    if !Path::new(SHELL_PATH).is_file() {
        return Err(format!("refusing to spawn terminal: {SHELL_PATH} not found").into());
    }

    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: DEFAULT_ROWS,
        cols: DEFAULT_COLS,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    // Fixed working directory for the session.
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());

    let mut cmd = CommandBuilder::new(SHELL_PATH);
    // Scrub inherited environment, then set only what an interactive shell needs.
    cmd.env_clear();
    cmd.env("PATH", SHELL_ENV_PATH);
    cmd.env("TERM", "xterm-256color");
    cmd.env("LANG", "C.UTF-8");
    cmd.env("HOME", &home);
    cmd.cwd(&home);
    let child = pair.slave.spawn_command(cmd)?;

    // Once bash holds the slave end we no longer need our copy of that fd;
    // dropping it lets the PTY report EOF when the shell exits.
    drop(pair.slave);

    Ok(Shell {
        master: pair.master,
        child,
    })
}

/// Drive a single terminal session for the lifetime of the WebSocket.
async fn handle_socket(socket: WebSocket) {
    // 1. Spawn the shell behind a PTY.
    let Shell { master, mut child } = match spawn_shell() {
        Ok(shell) => shell,
        Err(e) => {
            tracing::error!(error = %e, "terminal: failed to spawn shell");
            // Dropping `socket` closes the connection; we avoid holding the
            // non-Send error `e` across an `.await`.
            return;
        }
    };

    // Blocking reader/writer halves of the PTY master (cloned/taken before the
    // master is moved into the input task, where it is also used for resize).
    let mut reader = match master.try_clone_reader() {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "terminal: clone reader");
            return;
        }
    };
    let mut writer = match master.take_writer() {
        Ok(w) => w,
        Err(e) => {
            tracing::error!(error = %e, "terminal: take writer");
            return;
        }
    };

    // 2. Channels bridging the blocking PTY threads ↔ async tasks.
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Vec<u8>>(); // PTY → WS
    let (in_tx, mut in_rx) = mpsc::unbounded_channel::<Vec<u8>>(); // WS → PTY

    // PTY output → channel (blocking read loop on its own OS thread).
    std::thread::spawn(move || {
        let mut buf = [0u8; READ_BUF];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break, // EOF or error: the shell has exited.
                Ok(n) => {
                    if out_tx.send(buf[..n].to_vec()).is_err() {
                        break; // WS side gone.
                    }
                }
            }
        }
    });

    // channel → PTY input (blocking write loop on its own OS thread).
    std::thread::spawn(move || {
        while let Some(bytes) = in_rx.blocking_recv() {
            if writer.write_all(&bytes).is_err() || writer.flush().is_err() {
                break;
            }
        }
    });

    let (mut ws_sink, mut ws_stream) = socket.split();

    // 3a. Forward PTY output → WebSocket.
    let mut to_ws = tokio::spawn(async move {
        while let Some(chunk) = out_rx.recv().await {
            if ws_sink.send(Message::Binary(chunk.into())).await.is_err() {
                break;
            }
        }
        // The shell closed its output; politely close the socket.
        let _ = ws_sink.send(Message::Close(None)).await;
    });

    // 3b. Forward WebSocket input → PTY, handling control frames inline.
    let mut from_ws = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_stream.next().await {
            match msg {
                Message::Binary(data) => {
                    if in_tx.send(data.to_vec()).is_err() {
                        break; // writer thread gone.
                    }
                }
                Message::Text(text) => {
                    if let Ok(ControlMsg::Resize { cols, rows }) =
                        serde_json::from_str::<ControlMsg>(text.as_str())
                    {
                        let _ = master.resize(PtySize {
                            rows,
                            cols,
                            pixel_width: 0,
                            pixel_height: 0,
                        });
                    }
                }
                Message::Close(_) => break,
                _ => {} // Ping/Pong handled by axum.
            }
        }
    });

    // 4. When either direction ends, tear the other down and reap the shell.
    tokio::select! {
        _ = &mut to_ws => from_ws.abort(),
        _ = &mut from_ws => to_ws.abort(),
    }
    let _ = child.kill();
    let _ = tokio::task::spawn_blocking(move || child.wait()).await;
}
