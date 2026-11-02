// Pure grouping logic for the launcher (unit tested).
import type { AppWindow } from '../../lib/ipc';
import type { Pin } from './index';

export interface Group { key: string; path: string; name: string; pin?: Pin; windows: AppWindow[]; aumid?: string }

/** Program identity. Store apps' folders carry a version (…_1.2.3.0_x64__publisher) that
 *  changes on every update: drop it so a pin keeps matching the running app. */
export const keyOf = (path: string) =>
  path.toLowerCase().replace(/(\\windowsapps\\[^\\_]+)_[^\\]*?__/, '$1__');
/** A browser-installed web app (Chrome/Edge PWA) runs as the browser's exe but is its own
 *  app: its windows carry an app ID like "Chrome._crx_…". */
export const isWebApp = (aumid?: string | null) => !!aumid && aumid.includes('._crx_');
/** Group identity of a window or pin: web apps by their app ID, everything else by program. */
export const keyFor = (path: string, aumid?: string | null) => (isWebApp(aumid) ? `webapp:${aumid!.toLowerCase()}` : keyOf(path));
/** A pinned Store app and a window of it, by app ID. */
export const sameApp = (pin: Pin | undefined, w: AppWindow) => !!pin?.aumid && pin.aumid === w.aumid;
export const nameOf = (path: string) => path.split(/[\\/]/).pop()?.replace(/\.(exe|lnk)$/i, '') ?? path;

/** Pinned apps first (in pin order), then other running programs in first-seen order. */
export function group(pins: Pin[], windows: AppWindow[]): Group[] {
  const out = new Map<string, Group>();
  for (const pin of pins) { const k = keyFor(pin.path, pin.aumid); out.set(k, { key: k, path: pin.path, name: pin.name ?? nameOf(pin.path), pin, windows: [], aumid: pin.aumid }); }
  for (const w of windows) {
    if (!w.exe) continue;
    // A Store app pinned from the app list can name a launcher .exe (Spotify: SpotifyMigrator.exe)
    // while its window runs another: match those by app ID.
    const own = keyFor(w.exe, w.aumid);
    const k = out.has(own) ? own : ([...out.values()].find((g) => sameApp(g.pin, w))?.key ?? own);
    // A web app is named after its window ("YouTube Music"), not after the browser.
    if (!out.has(k)) out.set(k, { key: k, path: w.exe, name: isWebApp(w.aumid) ? w.title : nameOf(w.exe), windows: [] });
    const g = out.get(k)!;
    g.windows.push(w);
    g.aumid ??= w.aumid ?? undefined;
  }
  return [...out.values()];
}

/** Which window a click on a group should act on: cycle through windows if one is active. */
export function nextWindow(g: Group): AppWindow | undefined {
  const i = g.windows.findIndex((w) => w.active);
  return i >= 0 && g.windows.length > 1 ? g.windows[(i + 1) % g.windows.length] : g.windows[Math.max(i, 0)];
}

/** Keep keys in the order they were first seen: new ones go to the end, gone ones drop out.
 *  (Windows lists windows in z-order, which changes on every focus.) */
export function stableOrder(prev: string[], current: string[]): string[] {
  const now = new Set(current);
  const kept = prev.filter((k) => now.has(k));
  const known = new Set(kept);
  return [...kept, ...current.filter((k) => !known.has(k))];
}

/** Move `key` so it ends up at index `to` (unknown keys: no change). */
export function moveKey(order: string[], key: string, to: number): string[] {
  const i = order.indexOf(key);
  if (i < 0) return order;
  const rest = order.filter((k) => k !== key);
  rest.splice(Math.max(0, Math.min(to, rest.length)), 0, key);
  return rest;
}

/** Pins sorted by the user's saved icon order; pins not in it keep their place at the end. */
export function pinsInOrder(pins: Pin[], order: string[]): Pin[] {
  const at = (p: Pin) => { const i = order.indexOf(keyFor(p.path, p.aumid)); return i < 0 ? Infinity : i; };
  return [...pins].sort((a, b) => at(a) - at(b));
}

/** Which icons fit when the launcher shows at most `max` (0 = all): the first ones in order,
 *  but the active app always stays visible (it takes the last slot). `hidden` = how many
 *  more there are (the "+N" chip). */
export function limitIcons<T>(items: T[], max: number, isActive: (t: T) => boolean): { shown: T[]; hidden: number } {
  if (!max || items.length <= max) return { shown: items, hidden: 0 };
  const shown = items.slice(0, max);
  const active = items.findIndex(isActive);
  if (active >= max) shown[max - 1] = items[active];
  return { shown, hidden: items.length - max };
}

/** Minimized windows in the order they were minimized (each gets its own spot at the end of
 *  the launcher when windows don't go into their icon). */
export function minimizedOrder(prev: number[], windows: AppWindow[]): number[] {
  const now = new Set(windows.filter((w) => w.minimized).map((w) => w.id));
  const kept = prev.filter((id) => now.has(id));
  return [...kept, ...[...now].filter((id) => !kept.includes(id))];
}
