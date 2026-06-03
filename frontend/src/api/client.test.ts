// F1 — API Client tests. Mocks global `fetch` (vitest jsdom, globals enabled).

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { getCurrent, getHistory } from './client';
import type { CurrentResponse, HistoryResponse } from './types';

/** Build a minimal Response-like object for the mocked fetch. */
function fakeResponse(opts: {
  ok?: boolean;
  status: number;
  jsonBody?: unknown;
  textBody?: string;
}): Response {
  const ok = opts.ok ?? (opts.status >= 200 && opts.status < 300);
  const text =
    opts.textBody ??
    (opts.jsonBody === undefined ? '' : JSON.stringify(opts.jsonBody));
  return {
    ok,
    status: opts.status,
    json: () => Promise.resolve(opts.jsonBody),
    text: () => Promise.resolve(text),
  } as unknown as Response;
}

function lastFetchUrl(): string {
  const mock = globalThis.fetch as unknown as ReturnType<typeof vi.fn>;
  return String(mock.mock.calls[mock.mock.calls.length - 1][0]);
}

beforeEach(() => {
  globalThis.fetch = vi.fn();
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllEnvs();
});

describe('getCurrent', () => {
  it('builds the correct URL and parses a 200 JSON body into CurrentResponse', async () => {
    const payload: CurrentResponse = {
      timestamp: 1717400000,
      cpu: { percent: 23.5 },
      memory: { total_bytes: 17179869184, used_bytes: 8589934592, percent: 50.0 },
      network: { rx_bps: 125000.0, tx_bps: 34000.0 },
    };
    (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(
      fakeResponse({ status: 200, jsonBody: payload }),
    );

    const result = await getCurrent();

    expect(globalThis.fetch).toHaveBeenCalledTimes(1);
    expect(lastFetchUrl().endsWith('/api/metrics/current')).toBe(true);
    expect(result).toEqual(payload);
  });

  it('resolves to null on a 204 response', async () => {
    (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(
      fakeResponse({ status: 204, ok: true }),
    );

    const result = await getCurrent();

    expect(result).toBeNull();
  });

  it('resolves to null on a 200 with an empty body', async () => {
    (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(
      fakeResponse({ status: 200, textBody: '' }),
    );

    const result = await getCurrent();

    expect(result).toBeNull();
  });

  it('throws on a non-2xx response (500)', async () => {
    (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(
      fakeResponse({ status: 500, ok: false }),
    );

    await expect(getCurrent()).rejects.toThrow(/500/);
  });

  it('honors VITE_API_BASE when set', async () => {
    vi.stubEnv('VITE_API_BASE', 'http://localhost:8080');
    (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(
      fakeResponse({ status: 204, ok: true }),
    );

    await getCurrent();

    expect(lastFetchUrl()).toBe('http://localhost:8080/api/metrics/current');
  });
});

describe('getHistory', () => {
  it('sends the correct range query param and parses HistoryResponse', async () => {
    const payload: HistoryResponse = {
      range: '24h',
      bucket_secs: 240,
      points: [
        {
          timestamp: 1717310000,
          cpu_percent: 20.1,
          mem_percent: 48.0,
          net_rx_bps: 100000,
          net_tx_bps: 20000,
        },
      ],
    };
    (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(
      fakeResponse({ status: 200, jsonBody: payload }),
    );

    const result = await getHistory('24h');

    expect(globalThis.fetch).toHaveBeenCalledTimes(1);
    expect(lastFetchUrl()).toContain('/api/metrics/history?range=24h');
    expect(result).toEqual(payload);
  });

  it('throws on a non-2xx response (500)', async () => {
    (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(
      fakeResponse({ status: 500, ok: false }),
    );

    await expect(getHistory('1h')).rejects.toThrow(/500/);
  });
});
