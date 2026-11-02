import { describe, expect, it } from 'vitest';
import { latestOnly } from './ipc';

describe('latestOnly', () => {
  it('sends one at a time and skips values replaced while one was in flight', async () => {
    const sent: number[] = [];
    let release!: () => void;
    const q = latestOnly(async (v: number) => { sent.push(v); if (v === 1) await new Promise<void>((r) => (release = r)); });
    const first = q.push(1);
    q.push(2); q.push(3); q.push(4);
    expect(q.busy()).toBe(true);
    release();
    await first;
    expect(sent).toEqual([1, 4]);
    expect(q.busy()).toBe(false);
  });
});
