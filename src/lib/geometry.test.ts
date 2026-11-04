import { describe, expect, it } from 'vitest';
import { hiddenCounts, reorder, snap, indexAt, revealBy, wrapDir, unrotate } from './geometry';

const mons = [{ x: 0, y: 0, w: 1920, h: 1080 }, { x: 1920, y: 0, w: 1920, h: 1080 }];

describe('snap', () => {
  it('docks to the nearest edge within threshold', () => {
    expect(snap({ x: 400, y: 1080 - 60, w: 1000, h: 48 }, mons, 24)).toMatchObject({ monitor: 0, edge: 'bottom' });
    expect(snap({ x: 400, y: 10, w: 1000, h: 48 }, mons, 24)).toMatchObject({ monitor: 0, edge: 'top' });
  });
  it('floats when no edge is close', () => {
    expect(snap({ x: 400, y: 500, w: 1000, h: 48 }, mons, 24)).toMatchObject({ monitor: 0, edge: null });
  });
  it('picks the monitor holding the window center', () => {
    expect(snap({ x: 2000, y: 1030, w: 1000, h: 48 }, mons, 24)).toMatchObject({ monitor: 1, edge: 'bottom' });
  });
  it('docks to a side edge', () => {
    expect(snap({ x: 1920 + 1920 - 50, y: 200, w: 48, h: 700 }, mons, 24)).toMatchObject({ monitor: 1, edge: 'right' });
  });
  it('falls back to the nearest monitor when the center is off-screen', () => {
    expect(snap({ x: -900, y: 10, w: 1000, h: 48 }, mons, 24).monitor).toBe(0);
  });
});

describe('reorder', () => {
  it('moves an item forward and back', () => {
    expect(reorder(['a', 'b', 'c', 'd'], 0, 2)).toEqual(['b', 'c', 'a', 'd']);
    expect(reorder(['a', 'b', 'c', 'd'], 3, 0)).toEqual(['d', 'a', 'b', 'c']);
  });
  it('does not mutate and clamps the target', () => {
    const src = ['a', 'b'];
    expect(reorder(src, 0, 9)).toEqual(['b', 'a']);
    expect(src).toEqual(['a', 'b']);
  });
});

describe('indexAt', () => {
  it('finds the slot a pointer is over from item centers', () => {
    const centers = [50, 150, 250];
    expect(indexAt(centers, 10)).toBe(0);
    expect(indexAt(centers, 120)).toBe(1);
    expect(indexAt(centers, 999)).toBe(2);
  });
});

describe('snap: drag past the edge to auto-hide', () => {
  const one = [{ x: 0, y: 0, w: 1920, h: 1080 }];
  it('hides when the bar centre goes below the bottom edge', () => {
    expect(snap({ x: 400, y: 1060, w: 1000, h: 60 }, one, 24)).toEqual({ monitor: 0, edge: 'bottom', hide: true });
  });
  it('hides past the top and side edges too', () => {
    expect(snap({ x: 400, y: -40, w: 1000, h: 60 }, one, 24)).toMatchObject({ edge: 'top', hide: true });
    expect(snap({ x: -40, y: 200, w: 60, h: 700 }, one, 24)).toMatchObject({ edge: 'left', hide: true });
  });
  it('hides once a third of the bar is past any of the four edges', () => {
    // 60 px thick: 21 px past = hide, 15 px past = just snap
    expect(snap({ x: 400, y: -21, w: 1000, h: 60 }, one, 24)).toMatchObject({ edge: 'top', hide: true });
    expect(snap({ x: 400, y: 1080 - 60 + 21, w: 1000, h: 60 }, one, 24)).toMatchObject({ edge: 'bottom', hide: true });
    expect(snap({ x: -21, y: 200, w: 60, h: 700 }, one, 24)).toMatchObject({ edge: 'left', hide: true });
    expect(snap({ x: 1920 - 60 + 21, y: 200, w: 60, h: 700 }, one, 24)).toMatchObject({ edge: 'right', hide: true });
    expect(snap({ x: 400, y: -15, w: 1000, h: 60 }, one, 24)).toMatchObject({ edge: 'top', hide: false });
    expect(snap({ x: 1920 - 60 + 15, y: 200, w: 60, h: 700 }, one, 24)).toMatchObject({ edge: 'right', hide: false });
  });
  it('sliding a wide bar sideways off the screen does not hide it', () => {
    expect(snap({ x: 1300, y: 0, w: 1000, h: 60 }, one, 24)).toMatchObject({ edge: 'top', hide: false });
  });
  it('a bar on the second monitor hides at that monitor\'s edge', () => {
    const two = [{ x: -1920, y: 0, w: 1536, h: 864 }, { x: 0, y: 0, w: 1920, h: 1080 }];
    expect(snap({ x: -1700, y: -25, w: 1200, h: 60 }, two, 24)).toEqual({ monitor: 0, edge: 'top', hide: true });
  });
  it('does not hide when merely near the edge', () => {
    expect(snap({ x: 400, y: 1010, w: 1000, h: 60 }, one, 24)).toEqual({ monitor: 0, edge: 'bottom', hide: false });
  });
});

describe('insertIndex (free drag)', () => {
  it('counts the other widgets whose centre is before the pointer', async () => {
    const { insertIndex } = await import('./geometry');
    expect(insertIndex([50, 150, 250], 10)).toBe(0);
    expect(insertIndex([50, 150, 250], 120)).toBe(1);
    expect(insertIndex([50, 150, 250], 999)).toBe(3);
  });
});

describe('hiddenCounts', () => {
  const box = (top: number, bottom: number) => ({ left: 0, right: 40, top, bottom });
  it('counts items cut off at each end of a column', () => {
    const items = [box(-60, -10), box(-5, 30), box(35, 80), box(90, 130), box(120, 170)];
    expect(hiddenCounts(items, box(0, 125), true)).toEqual({ before: 2, after: 2 });
    expect(hiddenCounts([box(0, 40), box(45, 90)], box(0, 125), true)).toEqual({ before: 0, after: 0 });
  });
  it('uses left/right for a row', () => {
    const r = (left: number, right: number) => ({ left, right, top: 0, bottom: 40 });
    expect(hiddenCounts([r(-30, 10), r(20, 60), r(90, 140)], r(0, 100), false)).toEqual({ before: 1, after: 1 });
  });
});

describe('revealBy', () => {
  const view = { left: 0, right: 500, top: 0, bottom: 40 };
  const box = (left: number, right: number) => ({ left, right, top: 0, bottom: 40 });
  it('leaves a widget that already shows alone', () => {
    expect(revealBy(box(100, 300), view, false, 10)).toBe(0);
  });
  it('scrolls a widget that grew past the end into view', () => {
    expect(revealBy(box(300, 620), view, false, 10)).toBe(130);
  });
  it('scrolls back for one cut off at the start', () => {
    expect(revealBy(box(-50, 200), view, false, 10)).toBe(-60);
  });
  it('shows the start of one wider than the dock', () => {
    expect(revealBy(box(300, 1000), view, false, 10)).toBe(290);
  });
  it('works down a side dock', () => {
    expect(revealBy({ left: 0, right: 60, top: 700, bottom: 900 }, { left: 0, right: 60, top: 0, bottom: 800 }, true, 10)).toBe(110);
  });
});

describe('infinite scroll', () => {
  it('carries an end widget round only when it is wholly out of view and evens out the sides', () => {
    expect(wrapDir(300, 0, 100, 100)).toBe(1); // scrolled to the end: the first goes after the last
    expect(wrapDir(0, 300, 100, 100)).toBe(-1); // at the start: the last comes before the first
    expect(wrapDir(150, 100, 100, 100)).toBe(0); // already about mid-row
    expect(wrapDir(90, 0, 100, 100)).toBe(0); // overflow smaller than the widget: no room to carry it
  });
  it('settles: carrying one round never asks to carry it straight back', () => {
    let [before, after] = [400, 0];
    const w = 100;
    for (let i = 0; i < 10; i++) { const d = wrapDir(before, after, w, w); if (!d) break; before -= d * w; after += d * w; }
    expect([before, after]).toEqual([200, 200]);
  });
  it('maps the row as seen back to the saved order', () => {
    const saved = ['a', 'b', 'c', 'd'];
    const turn = 1; // shown as b c d a
    expect(unrotate(['b', 'c', 'd', 'a'], turn)).toEqual(saved);
    // c dropped in front of b: the saved order keeps the seam where it was
    expect(unrotate(['c', 'b', 'd', 'a'], turn)).toEqual(['a', 'c', 'b', 'd']);
    expect(unrotate(saved, 0)).toBe(saved);
  });
});
