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

/** Holidays for `state` ('' = national only) from `today` on, one per date and name. */
export function upcoming(items: Holiday[], state: string, today: string): Holiday[] {
  const seen = new Set<string>();
  return items.filter((h) => h.date >= today && (!h.states.length || (!!state && h.states.includes(state))))
    .filter((h) => { const k = h.date + h.name; return !seen.has(k) && !!seen.add(k); });
}

export function daysUntil(date: string, today: string): number {
  return Math.round((Date.parse(date) - Date.parse(today)) / 86_400_000);
}

export const whenLabel = (days: number) => (days === 0 ? 'Today' : days === 1 ? 'Tomorrow' : `in ${days} days`);
