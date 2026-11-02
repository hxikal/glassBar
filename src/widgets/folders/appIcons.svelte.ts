// Icons and running windows for pinned apps, shared by the folder tiles and the folder panel.
// The same calls the App Launcher makes; each window fetches an icon once per program.
import { events, ipc, type AppWindow } from '../../lib/ipc';
import type { Pin } from '../apps/index';
import { keyFor, sameApp } from '../apps/groups';
import { pinKey } from './folders';

const icons = $state<Record<string, string | null>>({});
const asked = new Set<string>();

/** The program's icon (null until it arrives, or if it has none). */
export function iconOf(p: Pin): string | null {
  const k = pinKey(p);
  if (!asked.has(k)) {
    asked.add(k);
    ipc.appIcon(p.path, null, p.aumid ?? null).then((src) => { if (src) icons[k] = src; }).catch(() => {});
  }
  return icons[k] ?? null;
}

/** Open windows, kept current. */
export function liveWindows() {
  let list = $state<AppWindow[]>([]);
  $effect(() => {
    ipc.appsList().then((w) => (list = w)).catch(() => {});
    const off = events.appsChanged((w) => (list = w));
    return () => { off.then((f) => f()); };
  });
  return {
    /** Windows of this pinned program, oldest first. */
    of: (p: Pin) => list.filter((w) => w.exe && (keyFor(w.exe, w.aumid) === pinKey(p) || sameApp(p, w))).sort((a, b) => a.id - b.id),
  };
}
