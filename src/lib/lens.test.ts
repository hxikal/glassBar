import { expect, test } from 'vitest';
import { lensMap } from './lens';

test('the lens bends only near the rim, always inward', () => {
  const w = 100, h = 40, d = lensMap(w, h, 12, 10).data;
  const px = (x: number, y: number) => [d[(y * w + x) * 4], d[(y * w + x) * 4 + 1]];
  expect(px(50, 20)).toEqual([128, 128]); // middle: no shift
  expect(px(1, 20)[0]).toBeGreaterThan(200); // left rim samples to the right
  expect(px(98, 20)[0]).toBeLessThan(60); // right rim samples to the left
  expect(px(50, 1)[1]).toBeGreaterThan(200); // top rim samples downward
  expect(px(50, 38)[1]).toBeLessThan(60); // bottom rim samples upward
  expect(px(20, 20)).toEqual([128, 128]); // past the band: none
});
