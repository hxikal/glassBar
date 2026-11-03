import { describe, expect, it } from 'vitest';
import { hijriDay, isoWeek, isWeekend, itemsByDay, monthCells } from './calendar';
import type { Countdown } from '../countdown/dates';

describe('calendar', () => {
  it('pads the month to whole Monday-first weeks', () => {
    const c = monthCells(new Date(2026, 10, 1)); // November 2026 starts on a Sunday
    expect(c.length).toBe(42);
    expect(c[0].key).toBe('2026-10-26');
    expect(c[0].inMonth).toBe(false);
    expect(c[6]).toMatchObject({ key: '2026-11-01', day: 1, inMonth: true });
    expect(c[41].key).toBe('2026-12-06');
  });
  it('numbers ISO weeks', () => {
    expect(isoWeek(new Date(2026, 0, 1))).toBe(1); // a Thursday
    expect(isoWeek(new Date(2027, 0, 1))).toBe(53); // a Friday, still 2026's last week
    expect(isoWeek(new Date(2026, 10, 3))).toBe(45);
  });
  it('knows which states rest on Friday', () => {
    const fri = new Date(2026, 10, 6), sun = new Date(2026, 10, 8);
    expect(isWeekend(fri, 'Kelantan')).toBe(true);
    expect(isWeekend(sun, 'Kelantan')).toBe(false);
    expect(isWeekend(sun, 'Selangor')).toBe(true);
    expect(isWeekend(fri, '')).toBe(false);
  });
  it('converts to Umm al-Qura', () => {
    expect(hijriDay(new Date(2024, 2, 11))).toEqual({ m: 9, d: 1 }); // 1 Ramadan 1445
  });
  it('groups holidays, countdowns and due to-dos by day', () => {
    const cd = { id: 'a', name: 'Deadline', date: '2026-11-10', emoji: '🎯' } as Countdown;
    const m = itemsByDay(
      [{ date: '2026-11-10', name: 'Deepavali', states: [] }],
      [{ id: 1, text: 'Pay bill', done: false, dueAt: new Date(2026, 10, 10, 9).getTime(), createdAt: 0 },
       { id: 2, text: 'No date', done: false, dueAt: null, createdAt: 0 }],
      [cd]);
    expect(m.get('2026-11-10')?.map((x) => x.kind)).toEqual(['holiday', 'countdown', 'todo']);
    expect(m.size).toBe(1);
  });
});
