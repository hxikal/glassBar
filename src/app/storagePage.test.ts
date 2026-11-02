import { describe, expect, it } from 'vitest';
import type { StorageRow } from '../lib/ipc';
import { fmtSize, sections, shares } from './storagePage';

const row = (group: StorageRow['group'], name: string, bytes: number, actions = 0): StorageRow =>
  ({ group, name, bytes, dbBytes: 0, note: '', items: [], actions: Array.from({ length: actions }, (_, i) => ({ id: `${i}`, label: '', detail: '', bytes: 0, destructive: false })) });

describe('storage page', () => {
  it('sorts groups and rows largest first and drops empty rows', () => {
    const s = sections([row('widget', 'Notes', 10), row('leftover', 'aiusage-web', 500), row('widget', 'Clipboard', 90),
                        row('widget', 'Timer', 0), row('app', 'Database', 40), row('widget', 'Lyrics', 0, 1)]);
    expect(s.map((g) => g.group)).toEqual(['leftover', 'widget', 'app']);
    expect(s[1].rows.map((r) => r.name)).toEqual(['Clipboard', 'Notes', 'Lyrics']);
    expect(s[1].bytes).toBe(100);
  });

  it('splits the bar by share, keeping small groups visible', () => {
    expect(shares([{ group: 'leftover', bytes: 999 }, { group: 'app', bytes: 1 }], 1000)).toEqual([{ group: 'leftover', pct: 99.9 }, { group: 'app', pct: 0.8 }]);
    expect(shares([{ group: 'app', bytes: 5 }], 0)).toEqual([{ group: 'app', pct: 0 }]);
  });

  it('formats sizes like Explorer', () => {
    expect(fmtSize(512)).toBe('512 bytes');
    expect(fmtSize(58953399)).toBe('56.2 MB');
    expect(fmtSize(2048)).toBe('2 KB');
  });
});
