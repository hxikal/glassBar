// Pure layout math for the bar (no DOM, no IPC) so it can be unit tested.
export type Edge = 'top' | 'bottom' | 'left' | 'right';
export interface Rect { x: number; y: number; w: number; h: number }

const dist2 = (r: Rect, px: number, py: number) => {
  const dx = Math.max(r.x - px, 0, px - (r.x + r.w));
  const dy = Math.max(r.y - py, 0, py - (r.y + r.h));
  return dx * dx + dy * dy;
};

/** Which monitor the window belongs to and the edge to dock to (null = float).
 *  `hide` = a third of the bar was dragged past that edge: auto-hide there. */
export function snap(win: Rect, monitors: Rect[], threshold: number): { monitor: number; edge: Edge | null; hide: boolean } {
  const cx = win.x + win.w / 2, cy = win.y + win.h / 2;
  let monitor = 0;
  monitors.forEach((m, i) => { if (dist2(m, cx, cy) < dist2(monitors[monitor], cx, cy)) monitor = i; });
  const m = monitors[monitor];
  // A third of the bar's thickness pushed past an edge counts as "push it off-screen".
  // A wide bar only hides off the top/bottom, a tall one off the sides, so sliding a bar
  // sideways towards another monitor never hides it by accident.
  const wide = win.w >= win.h;
  const past: Edge | null = wide
    ? (win.y + win.h > m.y + m.h + win.h / 3 ? 'bottom' : win.y < m.y - win.h / 3 ? 'top' : null)
    : (win.x < m.x - win.w / 3 ? 'left' : win.x + win.w > m.x + m.w + win.w / 3 ? 'right' : null);
  if (past) return { monitor, edge: past, hide: true };
  const gaps: [Edge, number][] = [
    ['top', win.y - m.y],
    ['bottom', m.y + m.h - (win.y + win.h)],
    ['left', win.x - m.x],
    ['right', m.x + m.w - (win.x + win.w)],
  ];
  const [edge, gap] = gaps.reduce((a, b) => (Math.abs(b[1]) < Math.abs(a[1]) ? b : a));
  return { monitor, edge: Math.abs(gap) <= threshold ? edge : null, hide: false };
}

/** Copy of `list` with the item at `from` moved to `to` (clamped). */
export function reorder<T>(list: T[], from: number, to: number): T[] {
  const out = list.slice();
  const [item] = out.splice(from, 1);
  out.splice(Math.max(0, Math.min(to, out.length)), 0, item);
  return out;
}

/** Index of the item whose center is nearest the pointer coordinate. */
export function indexAt(centers: number[], p: number): number {
  let best = 0;
  centers.forEach((c, i) => { if (Math.abs(c - p) < Math.abs(centers[best] - p)) best = i; });
  return best;
}

/** Free drag: where the dragged widget lands = how many of the *other* widgets'
 *  centres lie before the pointer. */
export function insertIndex(otherCenters: number[], p: number): number {
  return otherCenters.filter((c) => c < p).length;
}

type Box = { left: number; right: number; top: number; bottom: number };
/** How many items are cut off before and after the visible part of a scrolling row/column
 *  (partly hidden counts too). A 2 px tolerance absorbs sub-pixel layout. */
export function hiddenCounts(items: Box[], view: Box, vertical: boolean): { before: number; after: number } {
  const [a, b] = vertical ? (['top', 'bottom'] as const) : (['left', 'right'] as const);
  return {
    before: items.filter((r) => r[a] < view[a] - 2).length,
    after: items.filter((r) => r[b] > view[b] + 2).length,
  };
}

/**
 * How far to scroll a strip (`view`) so the whole of `item` shows, with `margin` px to spare
 * (the cut-off fade and "more" chip sit at the ends). 0 = it already shows. Wider than the
 * strip: line its start up with the strip's start, so the beginning is what you see.
 */
export function revealBy(item: { left: number; right: number; top: number; bottom: number },
                         view: { left: number; right: number; top: number; bottom: number }, vertical: boolean, margin = 0): number {
  const [a, b, va, vb] = vertical ? [item.top, item.bottom, view.top, view.bottom] : [item.left, item.right, view.left, view.right];
  if (b - a > vb - va - 2 * margin) return a - (va + margin);
  if (b > vb - margin) return b - (vb - margin);
  if (a < va + margin) return a - (va + margin);
  return 0;
}

/**
 * Infinite scroll: which end widget to carry round to the other end, so the view stays mid-row
 * and there is always more to scroll to. `before`/`after`: px of row hidden on each side;
 * `first`/`last`: the end widgets' size plus the gap. 1 = the first goes to the end, -1 = the
 * last to the front, 0 = leave it. Only moves a widget that is wholly out of view.
 */
export function wrapDir(before: number, after: number, first: number, last: number): -1 | 0 | 1 {
  if (before - after > first) return 1;
  if (after - before > last) return -1;
  return 0;
}

/** Infinite scroll: the row as seen (the saved order turned by `turn`) back to the saved order. */
export function unrotate<T>(seen: T[], turn: number): T[] {
  const k = seen.length - turn;
  return turn ? [...seen.slice(k), ...seen.slice(0, k)] : seen;
}
