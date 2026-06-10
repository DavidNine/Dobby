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

// --- Container details (GET /api/docker/containers/{id}) -------------------

/** One host binding of a container port (null host fields = exposed only). */
export interface DockerPortBinding {
  container_port: number;
  protocol: string;
  host_ip: string | null;
  host_port: number | null;
}

/** A filesystem mount on the container. */
export interface DockerMount {
  type: string | null;
  name: string | null;
  source: string | null;
  destination: string | null;
  mode: string | null;
  rw: boolean | null;
}

/** The container's attachment to one Docker network. */
export interface DockerNetwork {
  name: string;
  ip_address: string | null;
  gateway: string | null;
  mac_address: string | null;
}

/** GET /api/docker/containers/{id} — detail subset of `docker inspect`. */
export interface DockerContainerDetails {
  id: string;
  name: string;
  image: string;
  state: string;
  exit_code: number | null;
  /** RFC 3339 timestamps (started/finished are null when never run). */
  created: string | null;
  started_at: string | null;
  finished_at: string | null;
  restart_policy: string | null;
  restart_count: number;
  platform: string | null;
  command: string | null;
  working_dir: string | null;
  env: string[];
  labels: Record<string, string>;
  ports: DockerPortBinding[];
  mounts: DockerMount[];
  networks: DockerNetwork[];
}
