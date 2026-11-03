// Boost (perf/boost.rs): the words for its result.
import type { BoostReport, BoostSkip } from './ipc';

export const SKIP_TEXT: Record<BoostSkip, string> = {
  system: 'part of Windows',
  service: 'services',
  protected: 'protected',
  security: 'anti-virus or anti-cheat',
  elevated: 'running as administrator',
  neverTouch: 'on your never-touch list',
  foreground: 'the app in front',
  fullscreen: 'full screen',
  audio: 'playing sound or media',
  recent: 'used in the last 10 minutes',
  busy: 'busy working',
};

export function gb(n: number): string {
  return n >= 1073741824 ? `${(n / 1073741824).toFixed(1)} GB` : `${Math.round(n / 1048576)} MB`;
}

/** "RAM in use 9.8 GB → 9.1 GB (0.7 GB less)". A rise (an app grew meanwhile) is said as such. */
export function ramLine(r: BoostReport): string {
  const d = r.before - r.after;
  const change = Math.abs(d) < 1048576 ? 'about the same' : d > 0 ? `${gb(d)} less` : `${gb(-d)} more: apps grew meanwhile`;
  return `RAM in use ${gb(r.before)} → ${gb(r.after)} of ${gb(r.total)} (${change})`;
}

/** "Left alone: 120 part of Windows, 3 playing sound or media", largest first. */
export function skippedLine(r: BoostReport): string {
  return r.skipped.length ? `Left alone: ${r.skipped.map(([k, n]) => `${n} ${SKIP_TEXT[k]}`).join(', ')}.` : '';
}

/** The never-touch list as typed ("Spotify.exe, obs64") -> clean names. */
export const parseNames = (s: string) => [...new Set(s.split(/[,;\n]/).map((x) => x.trim()).filter(Boolean))];
