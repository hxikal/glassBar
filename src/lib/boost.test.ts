import { describe, expect, it } from 'vitest';
import type { BoostReport } from './ipc';
import { parseNames, ramLine, skippedLine } from './boost';

const G = 1073741824;
const r = (before: number, after: number, skipped: BoostReport['skipped'] = []): BoostReport =>
  ({ at: 0, before, after, total: 16 * G, trimmed: 0, trimmedBytes: 0, top: [], ownBytes: 0, lowered: [], skipped });

describe('boost words', () => {
  it('says how RAM in use changed, honestly', () => {
    expect(ramLine(r(9.8 * G, 9.1 * G))).toBe('RAM in use 9.8 GB → 9.1 GB of 16.0 GB (717 MB less)');
    expect(ramLine(r(8 * G, 8 * G + 1000))).toContain('about the same');
    expect(ramLine(r(8 * G, 8.5 * G))).toContain('512 MB more: apps grew meanwhile');
  });

  it('lists who was left alone', () => {
    expect(skippedLine(r(0, 0, [['system', 120], ['audio', 2]]))).toBe('Left alone: 120 part of Windows, 2 playing sound or media.');
    expect(skippedLine(r(0, 0))).toBe('');
  });

  it('reads the never-touch list', () => {
    expect(parseNames('Spotify.exe, obs64;\n, obs64 ')).toEqual(['Spotify.exe', 'obs64']);
  });
});
