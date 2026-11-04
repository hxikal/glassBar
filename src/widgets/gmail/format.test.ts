import { describe, expect, it } from 'vitest';
import { badge, hueOf, initial, mailTime } from './format';

describe('mailTime', () => {
  const now = new Date(2026, 10, 4, 15, 0).getTime();
  it('shows the time for today', () => {
    expect(mailTime(new Date(2026, 10, 4, 10, 30).getTime(), now, 'en-GB')).toBe('10:30');
  });
  it('says Yesterday', () => {
    expect(mailTime(new Date(2026, 10, 3, 23, 59).getTime(), now, 'en-GB')).toBe('Yesterday');
  });
  it('shows the weekday within a week', () => {
    expect(mailTime(new Date(2026, 10, 1, 9).getTime(), now, 'en-GB')).toBe('Sun');
  });
  it('shows the date for older mail, with the year when it differs', () => {
    expect(mailTime(new Date(2026, 9, 20).getTime(), now, 'en-GB')).toBe('20 Oct');
    expect(mailTime(new Date(2025, 9, 20).getTime(), now, 'en-GB')).toBe('20 Oct 2025');
  });
  it('is empty without a date', () => {
    expect(mailTime(0, now)).toBe('');
  });
});

describe('initial', () => {
  it('takes the first letter', () => {
    expect(initial('ali bin abu')).toBe('A');
    expect(initial('"Shopee"')).toBe('S');
    expect(initial('  ')).toBe('?');
  });
});

describe('hueOf', () => {
  it('is stable and ignores case', () => {
    expect(hueOf('ali@x.com')).toBe(hueOf('ALI@x.com'));
    expect(hueOf('ali@x.com')).toBeGreaterThanOrEqual(0);
    expect(hueOf('ali@x.com')).toBeLessThan(360);
  });
});

describe('badge', () => {
  it('hides zero and caps at 99+', () => {
    expect(badge(0)).toBe('');
    expect(badge(7)).toBe('7');
    expect(badge(150)).toBe('99+');
  });
});
