import { describe, expect, it } from 'vitest';
import { HOP_MS, settleBounces, startBounce } from './bounce';

const g = (key: string, ...ids: number[]) => ({ key, windows: ids.map((id) => ({ id })) });
const ending = (b: ReturnType<typeof settleBounces>, key: string) => b[key]?.ending !== undefined;

describe('launch bounce', () => {
  it('keeps bouncing while no new window has appeared', () => {
    const b = startBounce({}, 'code', [1], 0);
    expect(settleBounces(b, [g('code', 1)], 1000)).toBe(b); // unchanged: no re-render
  });
  it('finishes its hop once a new window shows up, then stops', () => {
    const b = settleBounces(startBounce({}, 'code', [1], 0), [g('code', 1, 7)], 1000);
    expect(ending(b, 'code')).toBe(true);
    expect(settleBounces(b, [g('code', 1, 7)], 1000 + HOP_MS - 1)).toHaveProperty('code');
    expect(settleBounces(b, [g('code', 1, 7)], 1000 + HOP_MS)).not.toHaveProperty('code');
  });
  it('gives up after 10 seconds', () => {
    const b = startBounce({}, 'tray-app', [], 0);
    expect(ending(settleBounces(b, [g('tray-app')], 10_001), 'tray-app')).toBe(true);
  });
  it('waits for the timeout when the app is not in the list yet', () => {
    const b = startBounce({}, 'new', [], 0);
    expect(ending(settleBounces(b, [], 5000), 'new')).toBe(false);
  });
  it('restarts the clock when launched again, even while ending', () => {
    const b = startBounce(settleBounces(startBounce({}, 'code', [], 0), [g('code')], 10_001), 'code', [], 10_100);
    expect(ending(settleBounces(b, [g('code')], 12_000), 'code')).toBe(false);
  });
});
