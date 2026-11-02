import { describe, expect, it } from 'vitest';
import { ago, changeList, fmtBytes, fmtMetric, fmtPct, netLastHour, pointAt, timeSeries } from './perfPage';
const s = (netBytes: number) => ({ at: 0, ramMb: 0, cpu: 0, gpu: null, netBytes, procs: [] });
describe('changeList', () => {
  const items = [{ name: 'Whole bar', perf: { low: 'paused' } }, { name: 'Equalizer', perf: { low: '15 fps', high: '60 fps' } }, { name: 'Clock' }, { name: 'Apps' }];
  it('lists what the mode changes, in order, and the rest A–Z', () => {
    expect(changeList('low', items)).toEqual({ rows: [{ name: 'Whole bar', text: 'paused' }, { name: 'Equalizer', text: '15 fps' }], unchanged: ['Apps', 'Clock'] });
  });
  it('a mode with no text for a widget leaves it unchanged', () => {
    expect(changeList('high', items).unchanged).toEqual(['Apps', 'Clock', 'Whole bar']);
  });
});
describe('formatting', () => {
  it('bytes', () => { expect(fmtBytes(348_000)).toBe('340 KB'); expect(fmtBytes(1.2 * 1048576)).toBe('1.2 MB'); });
  it('missing GPU shows a dash', () => { expect(fmtPct(null)).toBe('—'); expect(fmtPct(4.83)).toBe('4.8%'); });
  it('net in the last hour', () => { expect(netLastHour([s(100), s(400), s(1100)])).toBe(1000); expect(netLastHour([s(5)])).toBe(0); });
});
describe('timeSeries', () => {
  // 5 s samples while the page was closed, then 1 s samples once it opened: the graph must be
  // spaced by time, not by sample count.
  const at = (t: number, ramMb: number, netBytes: number) => ({ at: t, ramMb, cpu: 0, gpu: null, netBytes, procs: [] });
  const samples = [at(0, 100, 0), at(5_000, 110, 100), at(60_000, 200, 300), at(61_000, 205, 350), at(62_000, 210, 360)];
  it('one point per 30 s, the highest reading in it, gaps carry the last value', () => {
    expect(timeSeries(samples, 'ram', 30_000)).toEqual([110, 110, 210]);
  });
  it('network adds up what arrived in each 30 s', () => {
    expect(timeSeries(samples, 'net', 30_000)).toEqual([100, 0, 260]);
  });
  it('no samples, no points', () => { expect(timeSeries([], 'cpu', 30_000)).toEqual([]); });
});
describe('graph hover', () => {
  it('the pointer picks the nearest point across the width', () => {
    expect(pointAt(0, 500, 11)).toBe(0);
    expect(pointAt(500, 500, 11)).toBe(10);
    expect(pointAt(124, 500, 11)).toBe(2); // 124/50 = 2.48
    expect(pointAt(-20, 500, 11)).toBe(0); // off the left edge
    expect(pointAt(600, 500, 11)).toBe(10);
    expect(pointAt(250, 500, 1)).toBe(0);
  });
  it('each reading in its own unit', () => {
    expect(fmtMetric('ram', 312.4)).toBe('312 MB');
    expect(fmtMetric('cpu', 4.26)).toBe('4.3%');
    expect(fmtMetric('gpu', 0)).toBe('0.0%');
    expect(fmtMetric('net', 2048)).toBe('2 KB in 30 s');
  });
  it('how long ago, in words', () => {
    expect(ago(20_000)).toBe('just now');
    expect(ago(90_000)).toBe('1 min ago');
    expect(ago(45 * 60_000)).toBe('45 min ago');
  });
});
