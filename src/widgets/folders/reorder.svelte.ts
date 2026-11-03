// Hold (or start moving) to pick an item up in a grid; it takes the place of the nearest cell
// and the others make room. Esc puts it back. Used for the apps in a folder and for the
// folders in "All folders". `outside` = the pointer left the grid (apps: release to remove).
export function reorder<T>(key: (t: T) => string, cell: string, hold = 250) {
  const s = $state({ dragKey: null as string | null, order: null as T[] | null, outside: false });
  let grid: HTMLElement | undefined;
  let swallow = false;

  /** Move the item `k` to index `to` (clamped). */
  const move = (list: T[], k: string, to: number) => {
    const from = list.findIndex((x) => key(x) === k);
    if (from < 0) return list;
    const next = [...list];
    const [it] = next.splice(from, 1);
    next.splice(Math.max(0, Math.min(next.length, to)), 0, it);
    return next;
  };

  function down(e: PointerEvent, item: T, items: T[], done: (order: T[], outside: boolean, key: string) => void) {
    if (e.button !== 0) return;
    const start = { x: e.clientX, y: e.clientY };
    let picked = false;
    const pick = () => { picked = true; s.dragKey = key(item); s.order = [...items]; };
    const timer = setTimeout(pick, hold);
    const onMove = (ev: PointerEvent) => {
      if (!picked && Math.hypot(ev.clientX - start.x, ev.clientY - start.y) > 6) { clearTimeout(timer); pick(); }
      if (!picked || !s.order || !grid) return;
      const g = grid.getBoundingClientRect();
      s.outside = ev.clientX < g.left - 24 || ev.clientX > g.right + 24 || ev.clientY < g.top - 24 || ev.clientY > g.bottom + 24;
      if (s.outside) return;
      const d = [...grid.querySelectorAll<HTMLElement>(cell)].map((c) => { const r = c.getBoundingClientRect(); return Math.hypot(ev.clientX - (r.left + r.width / 2), ev.clientY - (r.top + r.height / 2)); });
      const to = d.indexOf(Math.min(...d));
      if (to >= 0 && to !== s.order.findIndex((x) => key(x) === s.dragKey)) s.order = move(s.order, s.dragKey!, to);
    };
    const finish = (keep: boolean) => {
      clearTimeout(timer);
      removeEventListener('pointermove', onMove);
      removeEventListener('pointerup', up);
      removeEventListener('keydown', esc);
      if (picked && keep && s.order) {
        swallow = !s.outside; // released outside: no click follows to swallow
        done(s.order, s.outside, s.dragKey!);
      }
      s.dragKey = null; s.order = null; s.outside = false;
    };
    const up = () => finish(true);
    const esc = (k: KeyboardEvent) => { if (k.key === 'Escape') finish(false); };
    addEventListener('pointermove', onMove);
    addEventListener('pointerup', up);
    addEventListener('keydown', esc);
  }

  return {
    get dragKey() { return s.dragKey; },
    get order() { return s.order; },
    get outside() { return s.outside; },
    set grid(el: HTMLElement | undefined) { grid = el; },
    get grid() { return grid; },
    /** True once for the click that ends a drag (so it doesn't open the item). */
    swallowClick() { const w = swallow; swallow = false; return w; },
    down,
  };
}
