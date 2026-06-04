// F2 — TypeScript types matching the Backend API contract (HLD §B6).
// Time = Unix epoch seconds; network = bytes/sec; range strings 1h/6h/24h/7d.

/** GET /api/metrics/current */
export interface CurrentResponse {
  timestamp: number;
  cpu: {
    percent: number;
  };
  memory: {
    total_bytes: number;
    used_bytes: number;
    percent: number;
  };
  network: {
    rx_bps: number;
    tx_bps: number;
  };
}

/** A single downsampled point in GET /api/metrics/history `points[]`. */
export interface HistoryPoint {
  timestamp: number;
  cpu_percent: number;
  mem_percent: number;
  net_rx_bps: number;
  net_tx_bps: number;
}

/** GET /api/metrics/history?range=1h|6h|24h|7d */
export interface HistoryResponse {
  range: string;
  bucket_secs: number;
  points: HistoryPoint[];
}

/** Valid `range` query parameter values. */
export type Range = '1h' | '6h' | '24h' | '7d';

// --- Docker container management (GET /api/docker/containers, B9) ----------

/** A published port mapping on a container. */
export interface DockerPort {
  private: number;
  public: number | null;
  type: string;
}

/** One container row. `state` is the canonical lowercase Docker state. */
export interface DockerContainer {
  id: string;
  name: string;
  image: string;
  state: string;
  status: string;
  created: number;
  ports: DockerPort[];
}

/** GET /api/docker/containers. `available` is false when the daemon is down. */
export interface DockerListResponse {
  available: boolean;
  error: string | null;
  containers: DockerContainer[];
}

/** Lifecycle actions accepted by POST /api/docker/containers/{id}/{action}. */
export type ContainerAction = 'restart' | 'start' | 'stop';
