<script lang="ts">
  // 3D mail icon with the unread count of every account as its badge (no extra text: the
  // badge says it). Click for the panel.
  import MailIcon3D from '../../lib/icons/MailIcon3D.svelte';
  import type { PillProps } from '../types';
  import type { GmailSettings } from './index';
  import { liveInboxes } from './inbox.svelte';
  import { accountsOf, nameOf } from './accounts';
  import { badge } from './format';
  import { ipc } from '../../lib/ipc';

  let { settings, vertical }: PillProps<GmailSettings> = $props();
  const accounts = $derived(accountsOf(settings));
  const box = liveInboxes(() => accounts.map((a) => a.email), () => settings.every);
  const states = $derived(accounts.map((a) => ({ a, s: box.by[a.email] })));
  const n = $derived(states.reduce((t, x) => t + (x.s?.i?.unread ?? 0), 0));
  const important = $derived(states.reduce((t, x) => t + (x.s?.i?.important ?? 0), 0));
  const none = $derived(!accounts.length || states.every((x) => x.s?.signedIn === false));
  const trouble = $derived(states.some((x) => x.s?.i?.cached || (x.s?.error && !x.s?.i)));
  // Realtime: Rust keeps one connection per signed-in account (repeat calls are free).
  const live = $derived(states.filter((x) => x.s?.signedIn).map((x) => x.a.email).join(','));
  $effect(() => { if (live) ipc.gmailWatch(live.split(',')).catch(() => {}); });

  const title = $derived(none ? 'Gmail: click to sign in'
    : 'Gmail\n' + states.map((x) => `${nameOf(x.a)}: ${x.s?.signedIn === false ? 'not signed in' : x.s?.error && !x.s?.i ? x.s.error : `${x.s?.i?.unread ?? 0} unread`}`).join('\n')
      + (important ? `\n${important} important` : ''));
</script>

<div class={vertical ? 'side' : 'pill'} {title}>
  <span class="ico">
    <MailIcon3D size={vertical ? 24 : 20} />
    {#if n}<b class="badge" class:imp={important}>{badge(n)}</b>{/if}
    {#if trouble}<span class="dot"></span>{/if}
  </span>
  {#if none}<span class={vertical ? 'side-c' : 'lbl'}>Sign in</span>{/if}
</div>

<style>
  .ico { position: relative; display: inline-grid; place-items: center; }
  .badge {
    position: absolute; top: -6px; right: -9px; min-width: 16px; height: 16px; padding: 0 4px;
    border-radius: 999px; background: #ff3b30; color: #fff; font-size: 9.5px; font-weight: 700; line-height: 16px;
    text-align: center; font-variant-numeric: tabular-nums; box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  }
  /* Something important is waiting: a soft pulse. */
  .badge.imp { animation: glow 2.4s ease-in-out infinite; }
  @keyframes glow { 50% { box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3), 0 0 0 3px rgba(255, 59, 48, 0.28); } }
  /* Offline or a sign-in problem: a small amber dot, bottom right. */
  .dot { position: absolute; right: -3px; bottom: -3px; width: 7px; height: 7px; border-radius: 50%; background: #ff9f0a; box-shadow: 0 0 0 1.5px var(--pill, #fff); }
  .lbl { font-size: 12px; margin-left: 4px; white-space: nowrap; }
</style>
