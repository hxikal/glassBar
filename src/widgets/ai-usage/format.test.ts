import { describe, expect, it } from 'vitest';
import { barAt, bothTip, fmtLeft, pctLabel, tone, fmtTokens, leftPct, modelName, pillRows, shownPct, tipLeft, visibleRows } from './format';

describe('ai usage format', () => {
  it('shortens token counts', () => {
    expect(fmtTokens(950)).toBe('950');
    expect(fmtTokens(12_345)).toBe('12K');
    expect(fmtTokens(1_234_567)).toBe('1.2M');
    expect(fmtTokens(2_500_000_000)).toBe('2.5B');
  });
  it('formats time to reset', () => {
    expect(fmtLeft(-5)).toBe('now');
    expect(fmtLeft(12 * 60_000)).toBe('12m');
    expect(fmtLeft((2 * 60 + 14) * 60_000)).toBe('2h 14m');
    expect(fmtLeft((3 * 1440 + 4 * 60 + 5) * 60_000)).toBe('3d 4h');
  });
  it('names models', () => {
    expect(modelName('claude-opus-5-5')).toBe('Opus 5.5');
    expect(modelName('claude-haiku-4-5-20251001')).toBe('Haiku 4.5');
    expect(modelName('claude-sonnet-4-20250514')).toBe('Sonnet 4');
    expect(modelName('<synthetic>')).toBe('<synthetic>');
  });
  it('maps pointer x to a bar, in order and clamped', () => {
    const seen = Array.from({ length: 31 }, (_, k) => barAt((k / 30) * 380, 380, 30));
    expect(seen[0]).toBe(0);
    expect(seen.at(-1)).toBe(29);
    expect(seen.every((v, k) => k === 0 || v! >= seen[k - 1]!)).toBe(true);
    expect(barAt(-5, 380, 30)).toBe(0);
    expect(barAt(10, 380, 0)).toBe(null);
    expect(tipLeft(0, 30)).toBeCloseTo(100 / 30);
    expect(tipLeft(29, 30)).toBeCloseTo((29 / 30) * 100);
  });
  it('builds pill rows: Session, Weekly, then per-model', () => {
    const r = pillRows([{ key: 'seven_day' }, { key: 'seven_day:Fable' }, { key: 'seven_day_opus' }]);
    expect(r.map((x) => [x.label, x.w?.key ?? null])).toEqual([
      ['Session', null], ['Weekly', 'seven_day'], ['Fable', 'seven_day:Fable'], ['Opus', 'seven_day_opus'],
    ]);
  });
  it('hides unticked rows but always keeps one', () => {
    const rows = [{ label: 'Session' }, { label: 'Weekly' }, { label: 'Fable' }];
    expect(visibleRows(rows, []).length).toBe(3);
    expect(visibleRows(rows, ['Session']).map((r) => r.label)).toEqual(['Weekly', 'Fable']);
    expect(visibleRows(rows, ['Session', 'Weekly']).map((r) => r.label)).toEqual(['Fable']);
    expect(visibleRows(rows, ['Session', 'Weekly', 'Fable']).map((r) => r.label)).toEqual(['Weekly']);
  });
  it('clamps percent left', () => {
    expect(leftPct(37.6)).toBe(62);
    expect(leftPct(130)).toBe(0);
    expect(shownPct(86.4, 'left')).toBe(14);
    expect(shownPct(86.4, 'used')).toBe(86);
  });
  it('labels and colours used vs left from the same usage', () => {
    expect(pctLabel(97, 'used')).toBe('97% used');
    expect(pctLabel(97, 'left')).toBe('3% left');
    expect(bothTip(97, '2d 4h')).toBe('97% used · 3% left · resets in 2d 4h');
    const RED = '#ff453a', AMBER = '#ff9f0a', GREEN = '#30d158';
    expect([tone(97), tone(90), tone(89.4), tone(70), tone(69.4), tone(0)]).toEqual([RED, RED, AMBER, AMBER, GREEN, GREEN]);
  });
});
