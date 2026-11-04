import { events, ipc, type Inbox } from '../../lib/ipc';

export interface AccountState { i: Inbox | null; error: string; loading: boolean; signedIn: boolean | null }

/** Every account's unread list, each checked every `every()` minutes. Rust caches for a
 *  minute, and every check or opened message is broadcast (gmail:changed), so the dock and
 *  the panel stay in step. */
export function liveInboxes(emails: () => string[], every: () => number) {
  const s = $state<{ by: Record<string, AccountState> }>({ by: {} });
  const slot = (e: string) => (s.by[e] ??= { i: null, error: '', loading: false, signedIn: null });

  const loadOne = async (e: string, force = false) => {
    const a = slot(e);
    a.signedIn = await ipc.gmailSignedIn(e).catch(() => false);
    if (!a.signedIn) { a.i = null; a.error = ''; return; }
    a.loading = true;
    try { a.i = await ipc.gmailInbox(e, force); a.error = a.i.error ?? ''; }
    catch (err) { a.error = String(err); }
    finally { a.loading = false; }
  };
  // One account at a time: Rust talks to Gmail one connection at a time anyway.
  const loadAll = async (force = false) => { for (const e of emails()) await loadOne(e, force); };

  $effect(() => {
    const list = emails();
    for (const k of Object.keys(s.by)) if (!list.includes(k)) delete s.by[k];
    loadAll();
    const t = setInterval(() => loadAll(), Math.max(1, every()) * 60_000);
    return () => clearInterval(t);
  });

  $effect(() => {
    const off = events.gmailChanged((i) => {
      if (!emails().includes(i.email)) return;
      if (!i.fetchedAt && !i.items.length) { loadOne(i.email); return; }  // signed in or out
      const a = slot(i.email);
      a.i = i;
      a.error = i.error ?? '';
    });
    return () => { off.then((f) => f()); };
  });

  return Object.assign(s, {
    refresh: () => loadAll(true),
    reload: (e: string) => loadOne(e),
    loading: () => Object.values(s.by).some((a) => a.loading),
  });
}
