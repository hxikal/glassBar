import { describe, expect, it } from 'vitest';
import { daysUntil, freshKeys, holidayKey, stateOf, upcoming, whenLabel } from './holidays';

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

describe('holiday dismiss keys', () => {
  const all = [
    { date: '2026-03-20', name: 'Hari Raya Puasa Holiday', states: [] },
    { date: '2026-03-21', name: 'Hari Raya Puasa', states: [] },
    { date: '2026-05-31', name: 'Wesak Day', states: [] },
    { date: '2026-06-01', name: "The Yang di-Pertuan Agong's Birthday", states: [] },
    { date: '2027-05-17', name: 'Hari Raya Haji (tentative)', states: [] },
    { date: '2027-05-18', name: 'Hari Raya Haji (Day 2) (tentative)', states: [] },
    { date: '2026-02-01', name: 'Thaipusam', states: [] },
    { date: '2026-02-02', name: 'Day off for Thaipusam', states: [] },
  ];
  const key = (i: number) => holidayKey(all, all[i]);
  it('gives every day of a multi-day holiday the first day\'s key', () => {
    expect(key(1)).toBe('2026-03-20_Hari Raya Puasa Holiday');
    expect(key(5)).toBe('2027-05-17_Hari Raya Haji (tentative)');
    expect(key(7)).toBe('2026-02-01_Thaipusam');
  });
  it('keeps different holidays on back-to-back days apart', () => {
    expect(key(3)).toBe("2026-06-01_The Yang di-Pertuan Agong's Birthday");
  });
  it('drops keys older than a year', () => {
    expect(freshKeys(['2025-11-02_A', '2025-11-03_B', '2026-12-25_Christmas Day'], '2026-11-03')).toEqual(['2025-11-03_B', '2026-12-25_Christmas Day']);
  });
});
