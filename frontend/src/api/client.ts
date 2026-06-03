// F1 — Typed HTTP client over `fetch` (HLD §4 F1, §B6).
// Wraps the Backend metrics endpoints and returns typed data.

import type { CurrentResponse, HistoryResponse, Range } from './types';

/**
 * Resolve the API base URL from the Vite env var `VITE_API_BASE`.
 * Falls back to '' (relative requests) when undefined so it works in
 * dev/test without configuration.
 */
export function getApiBase(): string {
  return import.meta.env.VITE_API_BASE ?? '';
}

/**
 * Shared fetch helper.
 * - Throws a descriptive Error on non-2xx responses (so the polling hook
 *   can surface a message).
 * - Returns the raw Response for callers that need to inspect status/body.
 */
async function request(path: string): Promise<Response> {
  const url = `${getApiBase()}${path}`;
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`Request to ${url} failed with HTTP ${res.status}`);
  }
  return res;
}

/**
 * GET /api/metrics/current
 * - HTTP 204 or an empty body → null (no data collected yet).
 * - non-2xx (other than 204) → throws.
 * - 2xx with body → parsed CurrentResponse.
 */
export async function getCurrent(): Promise<CurrentResponse | null> {
  const res = await request('/api/metrics/current');

  if (res.status === 204) {
    return null;
  }

  const body = await res.text();
  if (body.trim() === '') {
    return null;
  }

  return JSON.parse(body) as CurrentResponse;
}

/**
 * GET /api/metrics/history?range=<range>
 * - non-2xx → throws.
 * - 2xx → parsed HistoryResponse.
 */
export async function getHistory(range: Range): Promise<HistoryResponse> {
  const query = new URLSearchParams({ range }).toString();
  const res = await request(`/api/metrics/history?${query}`);
  return (await res.json()) as HistoryResponse;
}
