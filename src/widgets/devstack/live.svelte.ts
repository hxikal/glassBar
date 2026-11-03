// Laragon status, re-read every `ms` while the caller is mounted.
import { ipc, type DevStatus } from '../../lib/ipc';

export function liveStack(root: () => string, ms: number) {
  const s = $state<{ st: DevStatus | null }>({ st: null });
  const load = () => ipc.devStatus(root()).then((v) => (s.st = v)).catch(() => {});
  $effect(() => {
    root();
    load();
    const t = setInterval(load, ms);
    return () => clearInterval(t);
  });
  return { get st() { return s.st; }, load };
}

/** `fn()` re-run every `ms` while the caller is mounted (and when what it reads changes). */
export function poll<T>(fn: () => Promise<T>, ms: number) {
  const s = $state<{ v: T | null; err: string }>({ v: null, err: '' });
  const run = () => fn().then((v) => { s.v = v; s.err = ''; }).catch((e) => { s.err = String(e); });
  $effect(() => {
    run(); // reads the settings `fn` uses, so a change re-runs this
    const t = setInterval(run, ms);
    return () => clearInterval(t);
  });
  return { get v() { return s.v; }, get err() { return s.err; }, load: () => run() };
}

/** Folder → its last two parts, for one-line rows. */
export const shortDir = (d: string) => d.replace(/\\+$/, '').split('\\').slice(-2).join('\\');

/** "php-8.4.16-nts-Win32-vs17-x64" → "8.4.16 NTS"; "node-v24.12.0-win-x64" → "24.12.0". */
export function prettyVersion(folder: string) {
  const v = folder.match(/(\d+\.\d+(?:\.\d+)?)/)?.[1] ?? folder;
  return /-nts-/i.test(folder) ? `${v} NTS` : v;
}

/** Remembered per viewer (try/catch: storage can be off). */
export function remembered<T extends string>(key: string, fallback: T, ok: readonly T[]): T {
  try { const v = localStorage.getItem(key) as T | null; return v && ok.includes(v) ? v : fallback; } catch { return fallback; }
}
export function remember(key: string, v: string) { try { localStorage.setItem(key, v); } catch { /* fine */ } }
