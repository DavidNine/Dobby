import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useMetricsPolling, POLL_INTERVAL_MS } from './usePolling';
import * as client from '../api/client';
import type { CurrentResponse, HistoryResponse, Range } from '../api/types';

vi.mock('../api/client', () => ({
  getCurrent: vi.fn(),
  getHistory: vi.fn(),
}));

const getCurrent = vi.mocked(client.getCurrent);
const getHistory = vi.mocked(client.getHistory);

const sampleCurrent: CurrentResponse = {
  timestamp: 1000,
  cpu: { percent: 12 },
  memory: { total_bytes: 100, used_bytes: 50, percent: 50 },
  network: { rx_bps: 1, tx_bps: 2 },
};

const makeHistory = (range: string): HistoryResponse => ({
  range,
  bucket_secs: 60,
  points: [],
});

/** Set document.hidden and fire a visibilitychange event. */
function setHidden(hidden: boolean) {
  Object.defineProperty(document, 'hidden', {
    configurable: true,
    get: () => hidden,
  });
  document.dispatchEvent(new Event('visibilitychange'));
}

beforeEach(() => {
  vi.useFakeTimers();
  getCurrent.mockResolvedValue(sampleCurrent);
  getHistory.mockImplementation(async (range) => makeHistory(range));
  setHidden(false);
});

afterEach(() => {
  vi.clearAllMocks();
  vi.useRealTimers();
});

/** Flush pending microtasks/promises under fake timers. */
async function flush() {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
}

describe('useMetricsPolling', () => {
  it('fetches current and history once immediately on mount', async () => {
    const { result } = renderHook(() => useMetricsPolling('1h'));
    await flush();

    expect(getCurrent).toHaveBeenCalledTimes(1);
    expect(getHistory).toHaveBeenCalledTimes(1);
    expect(getHistory).toHaveBeenCalledWith('1h');

    expect(result.current.loading).toBe(false);
    expect(result.current.current).toEqual(sampleCurrent);
    expect(result.current.history).toEqual(makeHistory('1h'));
    expect(result.current.error).toBeNull();
    expect(result.current.lastUpdated).not.toBeNull();
  });

  it('re-fetches every POLL interval', async () => {
    renderHook(() => useMetricsPolling('1h'));
    await flush();
    expect(getCurrent).toHaveBeenCalledTimes(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    });
    expect(getCurrent).toHaveBeenCalledTimes(2);
    expect(getHistory).toHaveBeenCalledTimes(2);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    });
    expect(getCurrent).toHaveBeenCalledTimes(3);
    expect(getHistory).toHaveBeenCalledTimes(3);
  });

  it('re-fetches history with the new range when range changes', async () => {
    const { rerender } = renderHook(
      ({ range }: { range: Range }) => useMetricsPolling(range),
      { initialProps: { range: '1h' as Range } },
    );
    await flush();
    expect(getHistory).toHaveBeenCalledWith('1h');
    const callsBefore = getHistory.mock.calls.length;

    rerender({ range: '24h' });
    await flush();

    expect(getHistory.mock.calls.length).toBeGreaterThan(callsBefore);
    expect(getHistory).toHaveBeenLastCalledWith('24h');
  });

  it('pauses polling while the page is hidden and resumes when visible', async () => {
    renderHook(() => useMetricsPolling('1h'));
    await flush();
    expect(getCurrent).toHaveBeenCalledTimes(1);

    // Hide the page.
    await act(async () => {
      setHidden(true);
    });

    // Advancing time while hidden must not trigger fetches.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS * 3);
    });
    expect(getCurrent).toHaveBeenCalledTimes(1);
    expect(getHistory).toHaveBeenCalledTimes(1);

    // Becoming visible again fetches immediately and resumes polling.
    await act(async () => {
      setHidden(false);
    });
    await flush();
    expect(getCurrent).toHaveBeenCalledTimes(2);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    });
    expect(getCurrent).toHaveBeenCalledTimes(3);
  });

  it('sets error and retains previous data when the client throws', async () => {
    const { result } = renderHook(() => useMetricsPolling('1h'));
    await flush();
    expect(result.current.current).toEqual(sampleCurrent);
    const prevHistory = result.current.history;

    // Next poll rejects.
    getCurrent.mockRejectedValueOnce(new Error('boom'));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    });

    expect(result.current.error).toBeInstanceOf(Error);
    expect(result.current.error?.message).toBe('boom');
    // Previous data retained, not wiped.
    expect(result.current.current).toEqual(sampleCurrent);
    expect(result.current.history).toEqual(prevHistory);

    // A subsequent successful poll clears the error (still polling).
    await act(async () => {
      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    });
    expect(result.current.error).toBeNull();
  });

  it('does not update state after unmount', async () => {
    const { unmount } = renderHook(() => useMetricsPolling('1h'));
    await flush();
    unmount();
    // Advancing after unmount must not throw or warn (no interval running).
    await act(async () => {
      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS * 2);
    });
    // Only the single mount fetch happened.
    expect(getCurrent).toHaveBeenCalledTimes(1);
  });
});
