#!/usr/bin/env bash
#
# start.sh — launch the Dobby monitoring stack (backend + frontend) together.
#
# Starts the Rust backend (axum API on MONITOR_PORT, default 8080) and the
# Vite frontend dev server, then waits. Ctrl-C (or any exit) shuts both down.
#
# Usage:
#   ./start.sh                 # build + run both services
#   ./start.sh --release       # build/run the backend in release mode
#   ./start.sh --host          # bind the frontend to 0.0.0.0 for LAN access
#   ./start.sh --no-frontend   # backend only
#   ./start.sh --no-backend    # frontend only
#
# Common env overrides (see README.md for the full backend table):
#   MONITOR_PORT       backend API port        (default 8080)
#   MONITOR_BIND       backend bind address    (default 0.0.0.0)
#   MONITOR_DB_PATH    SQLite file             (default ./monitor.db)
#   VITE_API_BASE      frontend -> API base    (default http://localhost:8080)
#   FRONTEND_PORT      Vite dev server port    (default Vite's own default)

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$ROOT_DIR/backend"
FRONTEND_DIR="$ROOT_DIR/frontend"

# --- options ---------------------------------------------------------------
CARGO_PROFILE_FLAG=""
RUN_BACKEND=1
RUN_FRONTEND=1
FRONTEND_HOST=0

for arg in "$@"; do
  case "$arg" in
    --release)     CARGO_PROFILE_FLAG="--release" ;;
    --host)        FRONTEND_HOST=1 ;;
    --no-frontend) RUN_FRONTEND=0 ;;
    --no-backend)  RUN_BACKEND=0 ;;
    -h|--help)     grep -E '^#( |$)' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "Unknown option: $arg (try --help)" >&2; exit 1 ;;
  esac
done

# --- process tracking + cleanup -------------------------------------------
PIDS=()
CLEANED=0

cleanup() {
  [[ "$CLEANED" == "1" ]] && return
  CLEANED=1
  echo
  echo "[dobby] shutting down..."
  for pid in "${PIDS[@]}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done
  wait 2>/dev/null || true
  echo "[dobby] stopped."
}
trap cleanup EXIT INT TERM

# --- backend ---------------------------------------------------------------
if [[ "$RUN_BACKEND" == "1" ]]; then
  if [[ -f "$HOME/.cargo/env" ]]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
  fi
  if ! command -v cargo >/dev/null 2>&1; then
    echo "[dobby] cargo not found. Install Rust via https://rustup.rs first." >&2
    exit 1
  fi

  echo "[dobby] building backend ${CARGO_PROFILE_FLAG:-(debug)}..."
  ( cd "$BACKEND_DIR" && cargo build $CARGO_PROFILE_FLAG )

  echo "[dobby] starting backend on ${MONITOR_BIND:-0.0.0.0}:${MONITOR_PORT:-8080}"
  ( cd "$BACKEND_DIR" && exec cargo run $CARGO_PROFILE_FLAG ) &
  PIDS+=($!)
fi

# --- frontend --------------------------------------------------------------
if [[ "$RUN_FRONTEND" == "1" ]]; then
  if ! command -v npm >/dev/null 2>&1; then
    echo "[dobby] npm not found. Install Node 20+ first." >&2
    exit 1
  fi

  if [[ ! -d "$FRONTEND_DIR/node_modules" ]]; then
    echo "[dobby] installing frontend dependencies..."
    ( cd "$FRONTEND_DIR" && npm install )
  fi

  dev_args=()
  [[ "$FRONTEND_HOST" == "1" ]] && dev_args+=(--host)
  [[ -n "${FRONTEND_PORT:-}" ]] && dev_args+=(--port "$FRONTEND_PORT")

  echo "[dobby] starting frontend dev server${FRONTEND_HOST:+ (LAN)}"
  ( cd "$FRONTEND_DIR" && exec npm run dev -- "${dev_args[@]}" ) &
  PIDS+=($!)
fi

if [[ ${#PIDS[@]} -eq 0 ]]; then
  echo "[dobby] nothing to start (both services disabled)." >&2
  exit 1
fi

echo "[dobby] running. Press Ctrl-C to stop."
wait
