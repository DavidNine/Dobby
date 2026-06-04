// StatusLight — colored dot + label for a Docker container state.
//
// Pure presentational. The color is derived from the canonical lowercase Docker
// state via `stateColor` (exported for unit testing): green = running, amber =
// transitional, red = dead, slate = stopped/other.

export type StateTone = 'green' | 'amber' | 'red' | 'slate';

/** Map a Docker state string to a status tone. */
export function stateColor(state: string): StateTone {
  switch (state) {
    case 'running':
      return 'green';
    case 'restarting':
    case 'paused':
    case 'stopping':
      return 'amber';
    case 'dead':
      return 'red';
    // exited, created, removing, unknown, …
    default:
      return 'slate';
  }
}

const DOT_CLASS: Record<StateTone, string> = {
  green: 'bg-green-500',
  amber: 'bg-amber-500',
  red: 'bg-red-500',
  slate: 'bg-slate-400',
};

export interface StatusLightProps {
  /** Canonical lowercase Docker state (e.g. "running", "exited"). */
  state: string;
  /** Optional human-readable detail shown next to the label (e.g. "Up 2h"). */
  status?: string;
}

export default function StatusLight({ state, status }: StatusLightProps) {
  const tone = stateColor(state);
  return (
    <span className="inline-flex items-center gap-2">
      <span
        aria-hidden="true"
        className={`inline-block h-2.5 w-2.5 shrink-0 rounded-full ${DOT_CLASS[tone]}`}
      />
      <span className="capitalize">{state}</span>
      {status && (
        <span className="text-xs text-gray-500 dark:text-gray-400">{status}</span>
      )}
    </span>
  );
}
