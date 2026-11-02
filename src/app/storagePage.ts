// Settings › Storage: the pure helpers behind the page (storage.rs does the measuring).
import type { StorageGroup, StorageRow } from '../lib/ipc';

export const GROUP_TITLE: Record<StorageGroup, string> = {
  leftover: 'Leftovers · not used by any widget',
  widget: 'Widgets',
  app: 'App data',
  other: 'Other · not GlassBar\'s, left alone',
};

/** Groups with their rows, largest group first and largest row first; empty rows dropped. */
export function sections(rows: StorageRow[]) {
  const by = new Map<StorageGroup, StorageRow[]>();
  for (const r of rows) {
    if (r.bytes === 0 && !r.actions.length) continue;
    by.set(r.group, [...(by.get(r.group) ?? []), r]);
  }
  return [...by.entries()]
    .map(([group, list]) => ({ group, bytes: list.reduce((n, r) => n + r.bytes, 0), rows: [...list].sort((a, b) => b.bytes - a.bytes) }))
    .sort((a, b) => b.bytes - a.bytes);
}

/** Share of the total for the split bar, in %; tiny groups still get a visible sliver. */
export function shares(groups: { group: StorageGroup; bytes: number }[], total: number) {
  return groups.map((g) => ({ group: g.group, pct: total > 0 ? Math.max(g.bytes ? 0.8 : 0, (g.bytes * 100) / total) : 0 }));
}

export function fmtSize(n: number): string {
  if (n < 1024) return `${n} bytes`;
  if (n < 1048576) return `${Math.round(n / 1024)} KB`;
  if (n < 1073741824) return `${(n / 1048576).toFixed(1)} MB`;
  return `${(n / 1073741824).toFixed(1)} GB`;
}

export const fmtDay = (ms: number) => new Date(ms).toLocaleString([], { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });
