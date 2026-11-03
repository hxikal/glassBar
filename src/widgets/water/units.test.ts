import { describe, expect, it } from 'vitest';
import { fmtWater, progressOf, untilText } from './units';

describe('water units', () => {
  it('writes ml with thousands separators', () => {
    expect(fmtWater(750)).toBe('750 ml');
    expect(fmtWater(1250)).toBe('1,250 ml');
  });
  it('writes US fluid ounces', () => {
    expect(fmtWater(750, 'oz')).toBe('25 fl oz');
  });
  it('keeps progress between empty and full', () => {
    expect(progressOf(500, 2000)).toBe(0.25);
    expect(progressOf(2500, 2000)).toBe(1);
    expect(progressOf(100, 0)).toBe(0);
  });
});

describe('untilText', () => {
  it('says how long until the next reminder', () => {
    expect(untilText(-5000)).toBe('now');
    expect(untilText(30_000)).toBe('1m');
    expect(untilText(12 * 60_000)).toBe('12m');
    expect(untilText(65 * 60_000)).toBe('1h 05m');
  });
});
