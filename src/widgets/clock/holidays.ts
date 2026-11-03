// Malaysian public holidays for the clock: which apply to your state, and how soon.
import type { Holiday } from '../../lib/ipc';

/** State names as the holiday calendar spells them. */
export const STATES = ['Johor', 'Kedah', 'Kelantan', 'Malacca', 'Negeri Sembilan', 'Pahang', 'Penang', 'Perak', 'Perlis',
  'Sabah', 'Sarawak', 'Selangor', 'Terengganu', 'Kuala Lumpur', 'Labuan', 'Putrajaya'];
const ALIASES: [string, string][] = [['melaka', 'Malacca'], ['pulau pinang', 'Penang'], ['wilayah persekutuan labuan', 'Labuan'],
  ['wilayah persekutuan putrajaya', 'Putrajaya'], ['wilayah persekutuan', 'Kuala Lumpur'], ['federal territory of kuala lumpur', 'Kuala Lumpur']];

/** The state named in a place ("Shah Alam, Selangor, Malaysia"), or '' when none is. */
export function stateOf(place: string | undefined): string {
  const p = (place ?? '').toLowerCase();
  return ALIASES.find(([a]) => p.includes(a))?.[1] ?? STATES.find((s) => p.includes(s.toLowerCase())) ?? '';
}

/** Today's date as YYYY-MM-DD in local time. */
export const ymd = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;

/** Holidays for `state` ('' = national only), one per date and name. */
export function forState(items: Holiday[], state: string): Holiday[] {
  const seen = new Set<string>();
  return items.filter((h) => !h.states.length || (!!state && h.states.includes(state)))
    .filter((h) => { const k = h.date + h.name; return !seen.has(k) && !!seen.add(k); });
}

/** Those from `today` on. */
export const upcoming = (items: Holiday[], state: string, today: string) => forState(items, state).filter((h) => h.date >= today);

export function daysUntil(date: string, today: string): number {
  return Math.round((Date.parse(date) - Date.parse(today)) / 86_400_000);
}

export const whenLabel = (days: number) => (days === 0 ? 'Today' : days === 1 ? 'Tomorrow' : `in ${days} days`);

/** "Second Day of Chinese New Year", "Hari Raya Haji (Day 2) (tentative)", "Day off for Thaipusam"
 *  -> the holiday they belong to. */
const baseName = (n: string) => n.toLowerCase()
  .replace(/\((tentative|day \d)\)|day off for|second day of|observed|holiday|day \d|'s day|\bday\b/g, '')
  .replace(/\s+/g, ' ').trim();

const dayBefore = (d: string) => ymd(new Date(Date.parse(d + 'T12:00') - 86_400_000));

/** Dismiss key for `h`: date + name of the first day of its run, so the days of a multi-day
 *  holiday share one key. `all` = every holiday for the state (past ones too). */
export function holidayKey(all: Holiday[], h: Holiday): string {
  let cur = h;
  for (let prev; (prev = all.find((x) => x.date === dayBefore(cur.date) && baseName(x.name) === baseName(cur.name))); ) cur = prev;
  return `${cur.date}_${cur.name}`;
}

/** Dismissed keys still worth keeping: those from the last year. */
export const freshKeys = (keys: string[], today: string) => {
  const cut = `${Number(today.slice(0, 4)) - 1}${today.slice(4)}`;
  return keys.filter((k) => k.slice(0, 10) >= cut);
};
