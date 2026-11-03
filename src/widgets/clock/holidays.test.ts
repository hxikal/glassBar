import { describe, expect, it } from 'vitest';
import { daysUntil, stateOf, upcoming, whenLabel } from './holidays';

const items = [
  { date: '2026-02-01', name: 'Federal Territory Day', states: ['Kuala Lumpur', 'Labuan', 'Putrajaya'] },
  { date: '2026-08-31', name: "Malaysia's National Day", states: [] },
  { date: '2026-08-31', name: "Malaysia's National Day", states: [] },
  { date: '2026-12-11', name: "Sultan of Selangor's Birthday", states: ['Selangor'] },
];

describe('holidays', () => {
  it('keeps national ones plus your state, from today, without duplicates', () => {
    expect(upcoming(items, 'Selangor', '2026-01-01').map((h) => h.date)).toEqual(['2026-08-31', '2026-12-11']);
    expect(upcoming(items, '', '2026-01-01').map((h) => h.date)).toEqual(['2026-08-31']);
    expect(upcoming(items, 'Labuan', '2026-08-31').map((h) => h.date)).toEqual(['2026-08-31']);
  });
  it('finds the state in a place name', () => {
    expect(stateOf('Shah Alam, Selangor, Malaysia')).toBe('Selangor');
    expect(stateOf('Bandar Hilir, Melaka')).toBe('Malacca');
    expect(stateOf('Kuala Lumpur, Wilayah Persekutuan Kuala Lumpur')).toBe('Kuala Lumpur');
    expect(stateOf('Tokyo, Japan')).toBe('');
  });
  it('counts days', () => {
    expect(daysUntil('2026-11-08', '2026-11-03')).toBe(5);
    expect(whenLabel(0)).toBe('Today');
    expect(whenLabel(1)).toBe('Tomorrow');
    expect(whenLabel(5)).toBe('in 5 days');
  });
});
