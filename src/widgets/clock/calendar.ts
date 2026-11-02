// The Clock's month grid: its cells, week numbers, weekends, Hijri days, and what falls on
// each day (holidays, to-dos due, countdowns). Pure; tested in calendar.test.ts.
import type { Holiday, Todo } from '../../lib/ipc';
import type { Countdown } from '../countdown/dates';
import { ymd } from './holidays';

export interface Cell { date: Date; key: string; day: number; inMonth: boolean }

/** Monday-first weeks covering `month`, padded with the neighbouring months' days. */
export function monthCells(month: Date): Cell[] {
  const y = month.getFullYear(), m = month.getMonth();
  const lead = (new Date(y, m, 1).getDay() + 6) % 7;
  const days = new Date(y, m + 1, 0).getDate();
  const n = Math.ceil((lead + days) / 7) * 7;
  return Array.from({ length: n }, (_, i) => {
    const date = new Date(y, m, i + 1 - lead);
    return { date, key: ymd(date), day: date.getDate(), inMonth: date.getMonth() === m };
  });
}

/** ISO 8601 week number of `d`. */
export function isoWeek(d: Date): number {
  const t = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
  t.setUTCDate(t.getUTCDate() + 4 - (t.getUTCDay() || 7));
  return Math.ceil(((t.getTime() - Date.UTC(t.getUTCFullYear(), 0, 1)) / 86_400_000 + 1) / 7);
}

/** States whose weekend is Friday–Saturday (Johor moved to Saturday–Sunday in 2025). */
const FRI_SAT = ['Kedah', 'Kelantan', 'Terengganu'];
export function isWeekend(d: Date, state: string): boolean {
  const w = d.getDay();
  return FRI_SAT.includes(state) ? w === 5 || w === 6 : w === 0 || w === 6;
}

// One shared formatter: the grid asks for 42 days per render.
const hijriFmt = new Intl.DateTimeFormat('en-u-ca-islamic-umalqura', { month: 'numeric', day: 'numeric' });
/** Umm al-Qura month and day of `d`. */
export function hijriDay(d: Date): { m: number; d: number } {
  const p = hijriFmt.formatToParts(d);
  const n = (t: string) => parseInt(p.find((x) => x.type === t)?.value ?? '0', 10);
  return { m: n('month'), d: n('day') };
}

export interface Item { kind: 'holiday' | 'todo' | 'countdown'; label: string; emoji?: string; todo?: Todo }

/** What falls on each day (YYYY-MM-DD): holidays first, then countdowns, then to-dos due. */
export function itemsByDay(holidays: Holiday[], todos: Todo[], countdowns: Countdown[]): Map<string, Item[]> {
  const m = new Map<string, Item[]>();
  const put = (k: string, it: Item) => m.set(k, [...(m.get(k) ?? []), it]);
  holidays.forEach((h) => put(h.date, { kind: 'holiday', label: h.name }));
  countdowns.forEach((c) => put(c.date, { kind: 'countdown', label: c.name, emoji: c.emoji }));
  todos.forEach((t) => { if (t.dueAt != null) put(ymd(new Date(t.dueAt)), { kind: 'todo', label: t.text, todo: t }); });
  return m;
}
