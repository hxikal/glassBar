// Notification banners (queue) and the jump from a notification to its widget. Pure.

/** Which widget each notification source belongs to (and the part, for combined ones). */
export const SOURCE_WIDGET: Record<string, { widget: string; part?: 'pomodoro' | 'todo' | 'notes'; label: string }> = {
  timer: { widget: 'timer', label: 'Timer' },
  pomodoro: { widget: 'productivity', part: 'pomodoro', label: 'Pomodoro' },
  todo: { widget: 'productivity', part: 'todo', label: 'To-Do' },
  solat: { widget: 'solat', label: 'Prayer times' },
  ipu: { widget: 'ipu', label: 'Air quality' },
  downloads: { widget: 'downloads', label: 'Downloads' },
  bluetooth: { widget: 'quicksettings', label: 'Bluetooth battery' },
  countdown: { widget: 'countdown', label: 'Countdowns' },
  gmail: { widget: 'gmail', label: 'Gmail' },
};

export type JumpTarget =
  | { kind: 'widget'; instance: string; index: number; part?: 'pomodoro' | 'todo' | 'notes' }
  | { kind: 'center'; instance: string; index: number }
  | { kind: 'none' };

/** Where a click on a notification goes: its widget if on the bar; if it was removed (or
 *  has no widget), the Notification Center; else nowhere. Parts of a combined widget
 *  count only while that part is shown. */
export function jumpTarget(source: string, widgets: { id: string; instance: string; settings?: unknown }[]): JumpTarget {
  const m = SOURCE_WIDGET[source];
  if (m) {
    const index = widgets.findIndex((w) => w.id === m.widget);
    const s = (index >= 0 ? widgets[index].settings : null) as Record<string, unknown> | null;
    const partHidden = m.part && s && s[{ pomodoro: 'showPomo', todo: 'showTodo', notes: 'showNotes' }[m.part]] === false;
    if (index >= 0 && !partHidden) return { kind: 'widget', instance: widgets[index].instance, index, part: m.part };
  }
  const c = widgets.findIndex((w) => w.id === 'notifications');
  return c >= 0 ? { kind: 'center', instance: widgets[c].instance, index: c } : { kind: 'none' };
}

// ---- Banner queue ----

export interface Note { id: number; source: string; title: string; body: string; at: number; data: unknown }
export interface Banner extends Note {
  /** ends at (ms) while counting down */ until: number;
  /** ms left while paused (hover) */ left: number | null;
  /** beyond the 3 shown: not counting down yet */ waiting: boolean;
}
export const MAX_SHOWN = 3;

/** Shown ones count down; one that comes up from waiting starts its full time now. */
export const arm = (q: Banner[], now: number, ms: number): Banner[] =>
  q.map((b, i) => (i < MAX_SHOWN ? (b.waiting ? { ...b, waiting: false, until: now + ms } : b) : { ...b, waiting: true }));

/** Newest first. */
export const push = (q: Banner[], n: Note, now: number, ms: number): Banner[] =>
  q.some((b) => b.id === n.id) ? q : arm([{ ...n, until: now + ms, left: null, waiting: false }, ...q], now, ms);

/** Up to 3 on screen (newest on top), the rest counted as "+N more". */
export const shown = (q: Banner[]) => ({ list: q.slice(0, MAX_SHOWN), more: Math.max(0, q.length - MAX_SHOWN) });

/** Hovering pauses every banner's countdown; leaving resumes it where it was. */
export const pause = (q: Banner[], now: number): Banner[] => q.map((b) => (b.left === null ? { ...b, left: Math.max(0, b.until - now) } : b));
export const resume = (q: Banner[], now: number): Banner[] => q.map((b) => (b.left !== null ? { ...b, until: now + b.left, left: null } : b));

/** Drop shown ones whose time is up (paused or waiting ones never expire). */
export const expire = (q: Banner[], now: number, ms: number): Banner[] =>
  arm(q.filter((b) => b.waiting || b.left !== null || b.until > now), now, ms);
export const dismiss = (q: Banner[], id: number, now: number, ms: number) => arm(q.filter((b) => b.id !== id), now, ms);

/** When the next shown banner runs out (null: nothing counting down, so no timer). */
export function nextDeadline(q: Banner[]): number | null {
  const t = shown(q).list.filter((b) => b.left === null).map((b) => b.until);
  return t.length ? Math.min(...t) : null;
}
