// F4 — usePolling Hook (HLD §4 F4, §5(C)).
// Polls the metrics API every POLL_INTERVAL_MS, manages loading/error/data,
// and pauses while the page is hidden (resuming + refetching when visible).

import { useEffect, useRef, useState } from 'react';
import { getCurrent, getHistory } from '../api/client';
import type { CurrentResponse, HistoryResponse, Range } from '../api/types';

/** Polling interval in milliseconds (HLD: every 10s). */
export const POLL_INTERVAL_MS = 10_000;

export interface MetricsPollingState {
  current: CurrentResponse | null;
  history: HistoryResponse | null;
  loading: boolean;
  error: Error | null;
  /**
   * Unix epoch SECONDS of the last successful fetch, or null before the first
   * one. Seconds (not ms) so it matches the app-wide time contract and can be
   * passed straight to `formatTimestamp` (which does `new Date(ts * 1000)`).
   */
  lastUpdated: number | null;
}

/**
 * Poll the backend for current + history metrics.
 *
 * Loading semantics: `loading` is true only during the very first load (before
 * any data has arrived). Subsequent background polls do NOT flip `loading` back
 * to true, so the UI never flickers into a skeleton/empty state while it
 * already has data. Errors set `error` but retain the previous data and keep
 * polling.
 */
export function useMetricsPolling(range: Range): MetricsPollingState {
  const [current, setCurrent] = useState<CurrentResponse | null>(null);
  const [history, setHistory] = useState<HistoryResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [lastUpdated, setLastUpdated] = useState<number | null>(null);

  // Latest range, readable inside the interval callback without re-creating it.
  const rangeRef = useRef(range);
  rangeRef.current = range;

  // True while this hook instance is mounted; gates all setState calls so we
  // never update state after unmount or for a request fired by an old effect.
  const isMountedRef = useRef(true);

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  // (Re)start polling whenever the range changes. The effect fetches once
  // immediately, then sets up the interval and visibility handling.
  useEffect(() => {
    let intervalId: ReturnType<typeof setInterval> | null = null;

    const fetchOnce = async () => {
      try {
        const [c, h] = await Promise.all([
          getCurrent(),
          getHistory(rangeRef.current),
        ]);
        if (!isMountedRef.current) return;
        setCurrent(c);
        setHistory(h);
        setError(null);
        // Epoch SECONDS — formatTimestamp multiplies by 1000.
        setLastUpdated(Math.floor(Date.now() / 1000));
      } catch (err) {
        if (!isMountedRef.current) return;
        // Retain previous current/history; just surface the error.
        setError(err instanceof Error ? err : new Error(String(err)));
      } finally {
        if (isMountedRef.current) {
          setLoading(false);
        }
      }
    };

    const startInterval = () => {
      if (intervalId !== null) return;
      intervalId = setInterval(() => {
        void fetchOnce();
      }, POLL_INTERVAL_MS);
    };

    const stopInterval = () => {
      if (intervalId !== null) {
        clearInterval(intervalId);
        intervalId = null;
      }
    };

    const handleVisibility = () => {
      if (document.hidden) {
        // Pause polling while hidden.
        stopInterval();
      } else {
        // Resume: fetch immediately, then keep polling.
        void fetchOnce();
        startInterval();
      }
    };

    // Initial fetch + start polling (only if currently visible).
    void fetchOnce();
    if (!document.hidden) {
      startInterval();
    }

    document.addEventListener('visibilitychange', handleVisibility);

    return () => {
      stopInterval();
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, [range]);

  return { current, history, loading, error, lastUpdated };
}
