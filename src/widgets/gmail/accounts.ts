// Accounts, categories and what the panel shows (pure; tested in accounts.test.ts).
import type { Inbox, Mail, MailCategory } from '../../lib/ipc';

export interface Account { email: string; /** Shown on its tab; empty = the part before @. */ name: string }

/** The widget's accounts. Before multi-account there was one `email`; it becomes the first. */
export function accountsOf(s: { accounts?: Account[]; email?: string }): Account[] {
  if (s.accounts?.length) return s.accounts;
  return s.email ? [{ email: s.email.trim().toLowerCase(), name: '' }] : [];
}

export const nameOf = (a: Account) => a.name.trim() || a.email.split('@')[0];

const COLORS = ['#4f7cff', '#f59e0b', '#10b981', '#ec4899', '#8b5cf6', '#06b6d4'];
/** Each account keeps the colour of its place in the list. */
export const colorOf = (i: number) => COLORS[((i % COLORS.length) + COLORS.length) % COLORS.length];

/** A panel tab: one of Gmail's categories, or all of them together. */
export type Tab = MailCategory | 'all';

export const CATEGORIES: { id: Tab; name: string }[] = [
  { id: 'all', name: 'All mail' },
  { id: 'primary', name: 'Primary' },
  { id: 'promotions', name: 'Promotions' },
  { id: 'social', name: 'Social' },
  { id: 'updates', name: 'Updates' },
  { id: 'forums', name: 'Forums' },
];

/** All mail, Primary, Promotions and Social always; Updates and Forums only when they have mail. */
export const chipsFor = (counts: Partial<Record<Tab, number>>) =>
  CATEGORIES.filter((c) => ['all', 'primary', 'promotions', 'social'].includes(c.id) || (counts[c.id] ?? 0) > 0);

/** `categories.all` is every unread one, so a tab's count is always `categories[tab]`. */
export interface Totals { unread: number; important: number; categories: Partial<Record<Tab, number>> }

export function totals(inboxes: Inbox[]): Totals {
  const t: Totals = { unread: 0, important: 0, categories: {} };
  for (const i of inboxes) {
    t.unread += i.unread;
    t.important += i.important ?? 0;
    for (const [k, n] of Object.entries(i.categories ?? {}) as [MailCategory, number][]) t.categories[k] = (t.categories[k] ?? 0) + n;
  }
  if (t.unread) t.categories.all = t.unread;
  return t;
}

/** "12 new · 3 important · 9 promotions" (parts that are 0 are left out). */
export function summaryLine(t: Totals): string {
  if (!t.unread) return 'No unread email';
  const parts = [`${t.unread} new`];
  if (t.important) parts.push(`${t.important} important`);
  if (t.categories.promotions) parts.push(`${t.categories.promotions} promotions`);
  return parts.join(' · ');
}

export interface Tagged extends Mail { account: string }

/** One category's mail from the chosen inboxes: Gmail's important ones first (as highlighted
 *  cards), then the rest; each newest first. */
export function pick(inboxes: Inbox[], cat: Tab): { important: Tagged[]; rest: Tagged[] } {
  const all: Tagged[] = inboxes.flatMap((i) => i.items.filter((m) => cat === 'all' || (m.category || 'primary') === cat).map((m) => ({ ...m, account: i.email })));
  all.sort((a, b) => b.date - a.date || b.uid - a.uid);
  return { important: all.filter((m) => m.important), rest: all.filter((m) => !m.important) };
}
