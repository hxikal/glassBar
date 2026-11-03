// Formatting for the AI Usage widget.

/** 950 -> "950", 12_345 -> "12K", 1_234_567 -> "1.2M". */
export function fmtTokens(n: number): string {
  const units = ['', 'K', 'M', 'B'];
  let v = Math.max(0, n), i = 0;
  while (v >= 999.5 && i < units.length - 1) { v /= 1000; i++; }
  return (v < 9.95 && i > 0 ? v.toFixed(1) : String(Math.round(v))) + units[i];
}

/** Time until reset: "2h 14m", "3d 4h", "12m", "now". */
export function fmtLeft(ms: number): string {
  const m = Math.max(0, Math.round(ms / 60_000));
  if (m < 1) return 'now';
  const d = Math.floor(m / 1440), h = Math.floor((m % 1440) / 60), mm = m % 60;
  return d ? `${d}d ${h}h` : h ? `${h}h ${mm}m` : `${mm}m`;
}

/** Pill rows: Session (5-hour) and Weekly always, then each per-model weekly limit by model
 *  name. A row with no window means Anthropic reports no such limit right now. */
export function pillRows<W extends { key: string }>(windows: W[]): { label: string; w: W | null }[] {
  const get = (k: string) => windows.find((w) => w.key === k) ?? null;
  const models = windows
    .filter((w) => w.key.startsWith('seven_day:') || w.key === 'seven_day_opus' || w.key === 'seven_day_sonnet')
    .map((w) => ({ label: w.key.includes(':') ? w.key.split(':')[1] : w.key === 'seven_day_opus' ? 'Opus' : 'Sonnet', w }));
  return [{ label: 'Session', w: get('five_hour') }, { label: 'Weekly', w: get('seven_day') }, ...models];
}

/** Rows the user didn't untick. Settings store the hidden ones, so a new model limit shows up
 *  by default; at least one row always stays (Weekly if everything was unticked). */
export function visibleRows<R extends { label: string }>(rows: R[], hidden: string[]): R[] {
  const v = rows.filter((r) => !hidden.includes(r.label));
  return v.length ? v : rows.filter((r) => r.label === 'Weekly');
}

/** Which of `n` equal-width bars is under pointer offset `x` in a chart `width` wide. */
export const barAt = (x: number, width: number, n: number) =>
  n ? Math.max(0, Math.min(n - 1, Math.floor((x / width) * n))) : null;

/** Tooltip anchor: the bar's right edge on the left half, its left edge on the right half (% of width). */
export const tipLeft = (i: number, n: number) => ((i < n / 2 ? i + 1 : i) / n) * 100;

export const leftPct = (usedPct: number) => Math.max(0, Math.min(100, Math.round(100 - usedPct)));

/** The % the user chose to see: what's left (default) or what's used. */
export const shownPct = (usedPct: number, show: 'left' | 'used') => (show === 'used' ? 100 - leftPct(usedPct) : leftPct(usedPct));

/** Warning colour from what is used, whichever way it is shown: green under 70% used
 *  (over 30% left), amber to 89% (11–30% left), red from 90% (10% or less left). */
export function tone(usedPct: number): string {
  const u = 100 - leftPct(usedPct);
  return u >= 90 ? '#ff453a' : u >= 70 ? '#ff9f0a' : '#30d158';
}

/** "97% used" / "3% left": never a bare number. */
export const pctLabel = (usedPct: number, show: 'left' | 'used') => `${shownPct(usedPct, show)}% ${show}`;

/** Hover text with both sides: "97% used · 3% left · resets in 2d 4h". */
export const bothTip = (usedPct: number, resetIn: string) =>
  `${pctLabel(usedPct, 'used')} · ${pctLabel(usedPct, 'left')} · resets in ${resetIn}`;

/** "claude-opus-5-5" -> "Opus 5.5", unknown names kept. */
export function modelName(m: string): string {
  const x = /^claude-([a-z]+)-(\d+)(?:-(\d{1,2}))?(?:-\d{8})?$/.exec(m);
  return x ? `${x[1][0].toUpperCase()}${x[1].slice(1)} ${x[2]}${x[3] ? '.' + x[3] : ''}` : m;
}
