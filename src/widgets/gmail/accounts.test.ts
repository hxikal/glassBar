import { describe, expect, it } from 'vitest';
import type { Inbox, Mail } from '../../lib/ipc';
import { accountsOf, chipsFor, colorOf, nameOf, pick, summaryLine, totals } from './accounts';

const mail = (uid: number, date: number, extra: Partial<Mail> = {}): Mail => ({
  uid, date, fromName: 'a', fromAddr: 'a@x.com', subject: 's', snippet: '', messageId: '', important: false, category: 'primary', ...extra,
});
const inbox = (email: string, items: Mail[], extra: Partial<Inbox> = {}): Inbox => ({
  email, unread: items.length, important: 0, categories: {}, items, fetchedAt: 1, cached: false, error: null, ...extra,
});

describe('accountsOf', () => {
  it('turns the old single email into the first account', () => {
    expect(accountsOf({ email: ' Me@Gmail.com ' })).toEqual([{ email: 'me@gmail.com', name: '' }]);
  });
  it('prefers the account list and is empty with neither', () => {
    expect(accountsOf({ accounts: [{ email: 'a@x.com', name: 'Work' }], email: 'old@x.com' })).toEqual([{ email: 'a@x.com', name: 'Work' }]);
    expect(accountsOf({})).toEqual([]);
  });
});

describe('nameOf / colorOf', () => {
  it('uses the name, or the part before @', () => {
    expect(nameOf({ email: 'haikal@gmail.com', name: '' })).toBe('haikal');
    expect(nameOf({ email: 'haikal@gmail.com', name: ' Kerja ' })).toBe('Kerja');
  });
  it('cycles colours', () => {
    expect(colorOf(0)).toBe(colorOf(6));
    expect(colorOf(0)).not.toBe(colorOf(1));
  });
});

describe('totals and summaryLine', () => {
  it('adds up every account', () => {
    const t = totals([
      inbox('a', [], { unread: 8, important: 2, categories: { primary: 3, promotions: 5 } }),
      inbox('b', [], { unread: 4, important: 1, categories: { primary: 2, promotions: 2 } }),
    ]);
    expect(t).toEqual({ unread: 12, important: 3, categories: { primary: 5, promotions: 7 } });
    expect(summaryLine(t)).toBe('12 new · 3 important · 7 promotions');
  });
  it('leaves out zeros', () => {
    expect(summaryLine({ unread: 2, important: 0, categories: {} })).toBe('2 new');
    expect(summaryLine({ unread: 0, important: 0, categories: {} })).toBe('No unread email');
  });
});

describe('chipsFor', () => {
  it('shows Updates and Forums only when they have mail', () => {
    expect(chipsFor({}).map((c) => c.id)).toEqual(['primary', 'promotions', 'social']);
    expect(chipsFor({ forums: 1 }).map((c) => c.id)).toEqual(['primary', 'promotions', 'social', 'forums']);
  });
});

describe('pick', () => {
  it('merges accounts newest first, important ones apart, one category only', () => {
    const r = pick([
      inbox('a@x.com', [mail(1, 100), mail(2, 300, { important: true }), mail(3, 500, { category: 'promotions' })]),
      inbox('b@x.com', [mail(9, 200)]),
    ], 'primary');
    expect(r.important.map((m) => [m.account, m.uid])).toEqual([['a@x.com', 2]]);
    expect(r.rest.map((m) => [m.account, m.uid])).toEqual([['b@x.com', 9], ['a@x.com', 1]]);
  });
});
