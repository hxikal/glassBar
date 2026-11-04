// Launch bounce bookkeeping (pure, unit tested): an icon bounces from launch until its app
// shows a window it didn't have before, or for 10 s at most. Then it finishes the hop it is
// in (stopping mid-air snapped the icon back): the page drops it when the hop ends.
export type Bounces = Record<string, { since: number; ids: number[]; /** done since */ ending?: number }>;

export const BOUNCE_MS = 10_000;
/** One hop: the `hop` animation's length in Pill.svelte. */
export const HOP_MS = 600;

/** Start (or restart) bouncing `key`; `ids` = the windows it already had. */
export const startBounce = (b: Bounces, key: string, ids: number[], now: number): Bounces => ({ ...b, [key]: { since: now, ids } });

/** Mark the bounces that are done as ending, and drop ending ones whose last hop is surely
 *  over (no animation ran to say so). Returns `b` itself when nothing changed. */
export function settleBounces(b: Bounces, groups: { key: string; windows: { id: number }[] }[], now: number): Bounces {
  const out: Bounces = {};
  let changed = false;
  for (const [key, v] of Object.entries(b)) {
    if (v.ending !== undefined) {
      if (now - v.ending < HOP_MS) out[key] = v; else changed = true;
      continue;
    }
    const g = groups.find((x) => x.key === key);
    const done = g?.windows.some((w) => !v.ids.includes(w.id)) || now - v.since > BOUNCE_MS;
    out[key] = done ? { ...v, ending: now } : v;
    changed ||= !!done;
  }
  return changed ? out : b;
}
