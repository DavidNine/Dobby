// useDockerContainers — polls GET /api/docker/containers.
//
// Mirrors useMetricsPolling's lifecycle (poll on an interval, pause while the
// page is hidden, gate setState behind a mounted ref) but for the container
// list, and additionally exposes `refresh()` so the page can re-fetch
// immediately after a lifecycle action without waiting for the next tick.

import { useCallback, useEffect, useRef, useState } from 'react';
import { getContainers } from '../api/client';
import type { DockerListResponse } from '../api/types';

/** Container list poll interval (containers change faster than metrics). */
export const DOCKER_POLL_INTERVAL_MS = 5_000;

export interface DockerContainersState {
  data: DockerListResponse | null;
  /** True only during the very first load (before any data arrives). */
  loading: boolean;
  error: Error | null;
  /** Trigger an immediate out-of-band refetch. */
  refresh: () => void;
}

export function useDockerContainers(): DockerContainersState {
  const [data, setData] = useState<DockerListResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  const isMountedRef = useRef(true);
  // Bumped to force the polling effect to re-run an immediate fetch.
  const [refreshTick, setRefreshTick] = useState(0);
  const refresh = useCallback(() => setRefreshTick((t) => t + 1), []);

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    let intervalId: ReturnType<typeof setInterval> | null = null;

    const fetchOnce = async () => {
      try {
        const next = await getContainers();
        if (!isMountedRef.current) return;
        setData(next);
        setError(null);
      } catch (err) {
        if (!isMountedRef.current) return;
        // Retain previous data; just surface the error.
        setError(err instanceof Error ? err : new Error(String(err)));
      } finally {
        if (isMountedRef.current) setLoading(false);
      }
    };

    const startInterval = () => {
      if (intervalId !== null) return;
      intervalId = setInterval(() => void fetchOnce(), DOCKER_POLL_INTERVAL_MS);
    };
    const stopInterval = () => {
      if (intervalId !== null) {
        clearInterval(intervalId);
        intervalId = null;
      }
    };

    const handleVisibility = () => {
      if (document.hidden) {
        stopInterval();
      } else {
        void fetchOnce();
        startInterval();
      }
    };

    void fetchOnce();
    if (!document.hidden) startInterval();
    document.addEventListener('visibilitychange', handleVisibility);

    return () => {
      stopInterval();
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, [refreshTick]);

  return { data, loading, error, refresh };
}
