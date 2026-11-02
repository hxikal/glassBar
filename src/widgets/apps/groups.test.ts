import { describe, expect, it } from 'vitest';
import { group, keyFor, limitIcons, minimizedOrder, nextWindow, nameOf } from './groups';

const win = (id: number, exe: string, active = false) => ({ id, title: `w${id}`, exe, active, minimized: false });

describe('group', () => {
  it('puts pins first and merges running windows into them case-insensitively', () => {
    const g = group([{ path: 'C:\\Apps\\Code.exe' }], [win(1, 'C:\\Win\\notepad.exe'), win(2, 'c:\\apps\\code.exe')]);
    expect(g.map((x) => x.name)).toEqual(['Code', 'notepad']);
    expect(g[0].windows.map((w) => w.id)).toEqual([2]);
  });
  it('groups several windows of one program and skips unknown exes', () => {
    const g = group([], [win(1, 'a.exe'), win(2, ''), win(3, 'a.exe')]);
    expect(g).toHaveLength(1);
    expect(g[0].windows).toHaveLength(2);
  });
});

describe('nextWindow', () => {
  it('cycles when the group is active, else picks the first', () => {
    const [g] = group([], [win(1, 'a.exe'), win(2, 'a.exe', true), win(3, 'a.exe')]);
    expect(nextWindow(g)?.id).toBe(3);
    const [h] = group([], [win(1, 'a.exe'), win(2, 'a.exe')]);
    expect(nextWindow(h)?.id).toBe(1);
    const [solo] = group([], [win(7, 'a.exe', true)]);
    expect(nextWindow(solo)?.id).toBe(7); // same window: Rust minimizes it
  });
});

it('nameOf strips folders and extensions', () => {
  expect(nameOf('C:\\x\\Spotify.exe')).toBe('Spotify');
  expect(nameOf('C:/y/Chrome.lnk')).toBe('Chrome');
});

describe('stableOrder', () => {
  it('keeps the order apps were first seen, whatever Windows z-order says', async () => {
    const { stableOrder } = await import('./groups');
    let order: string[] = [];
    order = stableOrder(order, ['a', 'b', 'c']);
    expect(order).toEqual(['a', 'b', 'c']);
    order = stableOrder(order, ['c', 'a', 'b']); // focusing c moves it to the top of z-order
    expect(order).toEqual(['a', 'b', 'c']);
    order = stableOrder(order, ['d', 'a', 'c']); // b closed, d opened
    expect(order).toEqual(['a', 'c', 'd']);
  });
});

import { moveKey, pinsInOrder } from './groups';
describe('reorder', () => {
  it('moves a key to a new index', () => {
    expect(moveKey(['a', 'b', 'c', 'd'], 'a', 2)).toEqual(['b', 'c', 'a', 'd']);
    expect(moveKey(['a', 'b', 'c'], 'c', 0)).toEqual(['c', 'a', 'b']);
    expect(moveKey(['a', 'b'], 'x', 0)).toEqual(['a', 'b']);
  });
  it('sorts pins by the saved order, unknown pins last', () => {
    const pins = [{ path: 'A.exe' }, { path: 'B.exe' }, { path: 'C.exe' }];
    expect(pinsInOrder(pins, ['c.exe', 'zz.exe', 'a.exe']).map((p) => p.path)).toEqual(['C.exe', 'A.exe', 'B.exe']);
  });
});

import { keyOf as k } from './groups';
describe('store apps', () => {
  it('treat every version of a Store app as the same program', () => {
    expect(k('C:\\Program Files\\WindowsApps\\SpotifyAB.SpotifyMusic_1.306.1921.0_x64__zpdnekdrzrea0\\Spotify.exe'))
      .toBe(k('C:\\Program Files\\WindowsApps\\SpotifyAB.SpotifyMusic_1.400.0.0_x64__zpdnekdrzrea0\\Spotify.exe'));
    expect(k('C:\\Apps\\Code.exe')).toBe('c:\\apps\\code.exe');
  });
  it('carries the app identity from the pin or a running window', () => {
    const g = group([{ path: 'C:\\x\\a.exe', aumid: 'A!App' }], [{ id: 1, title: '', exe: 'C:\\y\\b.exe', active: false, minimized: false, aumid: 'B!App' }]);
    expect(g.map((x) => x.aumid)).toEqual(['A!App', 'B!App']);
  });
});

describe('installed web apps', () => {
  const chrome = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
  const app = 'Chrome._crx_agimnkijcaahngcdmfeangaknmldooml';
  const w = (id: number, title: string, aumid?: string) => ({ id, title, exe: chrome, active: false, minimized: false, aumid });
  it('are their own group, named after the app, not merged with the browser', () => {
    const g = group([{ path: chrome }], [w(1, 'Inbox - Gmail'), w(2, 'YouTube', app), w(3, 'Docs')]);
    expect(g.map((x) => [x.name, x.windows.map((y) => y.id)])).toEqual([['chrome', [1, 3]], ['YouTube', [2]]]);
    expect(g[1].aumid).toBe(app);
  });
  it('a pinned web app matches its running window', () => {
    const g = group([{ path: chrome, aumid: app, name: 'YouTube' }], [w(5, 'YouTube - Home', app)]);
    expect(g).toHaveLength(1);
    expect(g[0].windows.map((y) => y.id)).toEqual([5]);
    expect(keyFor(chrome, app)).not.toBe(keyFor(chrome));
  });
});

describe('limitIcons', () => {
  const on = (a: string) => (x: string) => x === a;
  it('shows the first N and counts the rest; 0 means all', () => {
    expect(limitIcons(['a', 'b', 'c', 'd', 'e'], 3, on(''))).toEqual({ shown: ['a', 'b', 'c'], hidden: 2 });
    expect(limitIcons(['a', 'b'], 3, on(''))).toEqual({ shown: ['a', 'b'], hidden: 0 });
    expect(limitIcons(['a', 'b', 'c'], 0, on(''))).toEqual({ shown: ['a', 'b', 'c'], hidden: 0 });
  });
  it('keeps the active app visible in the last slot', () => {
    expect(limitIcons(['a', 'b', 'c', 'd', 'e'], 3, on('e'))).toEqual({ shown: ['a', 'b', 'e'], hidden: 2 });
  });
});

describe('minimizedOrder', () => {
  const min = (id: number, minimized = true) => ({ ...win(id, 'a.exe'), minimized });
  it('lists minimized windows in the order they were minimized', () => {
    expect(minimizedOrder([1, 2], [min(2), min(1), min(3)])).toEqual([1, 2, 3]);
  });
  it('drops a window once it is restored', () => {
    expect(minimizedOrder([1, 2, 3], [min(1), min(2, false), min(3)])).toEqual([1, 3]);
  });
  it('drops a window once it is closed', () => {
    expect(minimizedOrder([1], [])).toEqual([]);
  });
  it('never lists windows that are not minimized', () => {
    expect(minimizedOrder([], [min(4, false)])).toEqual([]);
  });
});

describe('Store apps pinned from the app list', () => {
  it('match their window by app ID when the pinned .exe is a launcher', () => {
    const id = 'SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify';
    const dir = 'C:\Program Files\WindowsApps\SpotifyAB.SpotifyMusic_1.3.0.0_x64__zpdnekdrzrea0';
    const g = group([{ path: `${dir}\SpotifyMigrator.exe`, aumid: id, name: 'Spotify' }],
      [{ id: 7, title: 'Spotify', exe: `${dir}\Spotify.exe`, active: false, minimized: false, aumid: id }]);
    expect(g.length).toBe(1);
    expect(g[0].windows.map((w) => w.id)).toEqual([7]);
    expect(g[0].name).toBe('Spotify');
  });
});
