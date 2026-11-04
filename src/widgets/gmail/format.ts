// Small pure helpers for the Gmail widget (tested in format.test.ts).

/** "10:30" today, "Yesterday", "Mon" this week, else "3 Nov" (or "3 Nov 2025" another year). */
export function mailTime(at: number, now = Date.now(), locale?: string): string {
  if (!at) return '';
  const d = new Date(at), n = new Date(now);
  const day = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const days = Math.round((day(n) - day(d)) / 86_400_000);
  if (days <= 0) return d.toLocaleTimeString(locale, { hour: 'numeric', minute: '2-digit' });
  if (days === 1) return 'Yesterday';
  if (days < 7) return d.toLocaleDateString(locale, { weekday: 'short' });
  return d.toLocaleDateString(locale, d.getFullYear() === n.getFullYear() ? { day: 'numeric', month: 'short' } : { day: 'numeric', month: 'short', year: 'numeric' });
}

/** The letter shown in the sender's circle. */
export function initial(name: string): string {
  const c = name.trim().replace(/^["'<(]+/, '').charAt(0);
  return c ? c.toUpperCase() : '?';
}

/** A stable hue (0..359) per sender, so each person keeps their colour. */
export function hueOf(key: string): number {
  let h = 0;
  for (const ch of key.toLowerCase()) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
  return h % 360;
}

/** The dock badge: "99+" past 99, nothing at 0. */
export function badge(n: number): string {
  return n <= 0 ? '' : n > 99 ? '99+' : String(n);
}
