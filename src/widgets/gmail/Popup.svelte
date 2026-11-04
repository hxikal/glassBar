<script lang="ts">
  // Smart inbox: a summary card (new / important / promotions) with a tab per account, Gmail's
  // categories as chips, important mail first as cards, the rest as compact rows. Click one to
  // read it here (text only); it then leaves the list but stays unread in Gmail.
  import { ArrowLeft, ExternalLink, Plus, RefreshCw, Star } from '@lucide/svelte';
  import PanelHeader from '../../lib/PanelHeader.svelte';
  import { ipc, type MailBody, type MailCategory } from '../../lib/ipc';
  import { relTime } from '../../lib/time';
  import { updateWidgetSettings } from '../../lib/config.svelte';
  import type { PopupProps } from '../types';
  import type { GmailSettings } from './index';
  import { liveInboxes } from './inbox.svelte';
  import { accountsOf, chipsFor, colorOf, nameOf, pick, summaryLine, totals, type Tagged } from './accounts';
  import { hueOf, initial, mailTime } from './format';
  import SignIn from './SignIn.svelte';

  let { instance, settings }: PopupProps<GmailSettings> = $props();
  const accounts = $derived(accountsOf(settings));
  const box = liveInboxes(() => accounts.map((a) => a.email), () => settings.every);

  let tab = $state<string>('all');
  let cat = $state<MailCategory>('primary');
  let adding = $state<{ email: string } | null>(null);
  let open = $state<{ m: Tagged; b: MailBody | null; error: string } | null>(null);

  $effect(() => { if (tab !== 'all' && !accounts.some((a) => a.email === tab)) tab = 'all'; });

  const color = (email: string) => colorOf(accounts.findIndex((a) => a.email === email));
  const shown = $derived(accounts
    .filter((a) => tab === 'all' || a.email === tab)
    .map((a) => box.by[a.email]?.i)
    .filter((i) => !!i));
  const t = $derived(totals(shown));
  const chips = $derived(chipsFor(t.categories));
  const sel = $derived(pick(shown, cat));
  const problems = $derived(accounts
    .map((a) => ({ a, s: box.by[a.email] }))
    .filter((x) => x.s && (x.s.signedIn === false || (x.s.error && (!x.s.i || x.s.i.cached)))));
  const checkedAt = $derived(Math.min(...shown.map((i) => i.fetchedAt).filter(Boolean)));
  const offline = $derived(shown.some((i) => i.cached));
  const first = $derived(accounts.length && box.by[accounts[0].email]?.signedIn == null);

  const unreadOf = (email: string) => box.by[email]?.i?.unread ?? 0;
  const read = async (m: Tagged) => {
    open = { m, b: null, error: '' };
    try { const b = await ipc.gmailOpen(m.account, m.uid); if (open?.m === m) open.b = b; }
    catch (e) { if (open?.m === m) open.error = String(e); }
  };
  const web = (m: Tagged) => ipc.gmailOpenWeb(m.account, m.uid).catch(() => {});
  const added = (email: string) => {
    if (!accounts.some((a) => a.email === email)) updateWidgetSettings(instance, { accounts: [...accounts, { email, name: '' }], email: '' });
    else box.reload(email);
    adding = null;
  };
  const fullDate = (at: number) => at ? new Date(at).toLocaleString(undefined, { weekday: 'short', day: 'numeric', month: 'short', hour: 'numeric', minute: '2-digit' }) : '';
</script>

{#snippet avatar(m: Tagged, px: number)}
  <span class="av" style:--h={hueOf(m.fromAddr || m.fromName)} style:width="{px}px" style:height="{px}px" style:font-size="{Math.round(px * 0.4)}px">
    {initial(m.fromName)}
    {#if accounts.length > 1}<i style:background={color(m.account)} title={m.account}></i>{/if}
  </span>
{/snippet}

{#if open}
  <!-- Reading one email -->
  <PanelHeader title="Gmail">
    <button class="icon-btn" onclick={() => web(open!.m)} aria-label="Open in Gmail" title="Open in Gmail"><ExternalLink size={15} /></button>
  </PanelHeader>
  <button class="back" onclick={() => (open = null)}><ArrowLeft size={14} /> Back</button>
  <article class="msg">
    <h4>{#if open.m.important}<Star size={14} class="star" />{/if}{open.b?.subject ?? open.m.subject}</h4>
    <div class="meta">
      <span class="ellipsis"><b>{open.m.fromName}</b>{#if open.m.fromAddr && open.m.fromAddr !== open.m.fromName}<span class="dim"> &lt;{open.m.fromAddr}&gt;</span>{/if}</span>
      <span class="dim">{fullDate(open.b?.date || open.m.date)}{#if accounts.length > 1} · <span class="acc-dot" style:background={color(open.m.account)}></span>{nameOf(accounts.find((a) => a.email === open!.m.account) ?? { email: open.m.account, name: '' })}{/if}</span>
    </div>
    {#if open.error}
      <p class="err">{open.error}</p>
    {:else if !open.b}
      <div class="text skeleton"><i></i><i></i><i></i><i style="width:60%"></i></div>
    {:else}
      <div class="text">{open.b.text || '(This email has no text.)'}</div>
      {#if open.b.truncated}<p class="note">This email is long; GlassBar shows the start. Open it in Gmail for the rest and attachments.</p>{/if}
    {/if}
    <button class="ghost-btn" onclick={() => web(open!.m)}><ExternalLink size={13} /> Open in Gmail</button>
  </article>

{:else if adding || !accounts.length}
  <!-- Adding (or re-signing) an account -->
  <PanelHeader title={accounts.length ? 'Add Gmail account' : 'Gmail'} />
  {#if accounts.length}<button class="back" onclick={() => (adding = null)}><ArrowLeft size={14} /> Back</button>{/if}
  <SignIn email={adding?.email ?? ''} onSaved={added} />

{:else}
  <!-- The smart inbox -->
  <PanelHeader title="Gmail">
    <button class="icon-btn" onclick={() => (adding = { email: '' })} aria-label="Add account" title="Add account"><Plus size={16} /></button>
    <button class="icon-btn" class:spin={box.loading()} onclick={() => box.refresh()} aria-label="Check now" title="Check now"><RefreshCw size={15} /></button>
  </PanelHeader>

  <section class="hero" class:quiet={!t.unread}>
    <div class="n">{t.unread}</div>
    <div class="s">{summaryLine(t)}</div>
    {#if accounts.length > 1}
      <div class="accs" role="tablist">
        <button role="tab" aria-selected={tab === 'all'} class:on={tab === 'all'} onclick={() => (tab = 'all')}>All</button>
        {#each accounts as a, i (a.email)}
          <button role="tab" aria-selected={tab === a.email} class:on={tab === a.email} onclick={() => (tab = a.email)} title={a.email}>
            <span class="acc-dot" style:background={colorOf(i)}></span>{nameOf(a)} · {unreadOf(a.email)}
          </button>
        {/each}
      </div>
    {:else}
      <div class="who">{accounts[0].email}</div>
    {/if}
  </section>

  {#each problems as p (p.a.email)}
    <div class="warn">
      <span class="ellipsis">{p.s.signedIn === false ? `${p.a.email} isn't signed in` : `${nameOf(p.a)}: ${p.s.error}`}</span>
      {#if p.s.signedIn === false || p.s.error.includes('App Password')}<button onclick={() => (adding = { email: p.a.email })}>Sign in</button>{/if}
    </div>
  {/each}

  <div class="chips" role="tablist">
    {#each chips as c (c.id)}
      <button role="tab" aria-selected={cat === c.id} class:on={cat === c.id} onclick={() => (cat = c.id)}>
        {#if c.id === 'primary'}<Star size={11} />{/if}{c.name}{#if t.categories[c.id]}<b>{t.categories[c.id]}</b>{/if}
      </button>
    {/each}
  </div>

  <div class="list">
    {#if first && !shown.length}
      {#each [0, 1, 2] as k (k)}<div class="row skeleton"><span class="av"></span><span class="body"><i></i></span></div>{/each}
    {:else}
      {#each sel.important as m (m.account + m.uid)}
        <button class="imp" onclick={() => read(m)}>
          {@render avatar(m, 34)}
          <span class="body">
            <span class="top"><b class="ellipsis">{m.fromName}</b><span class="time">{mailTime(m.date)}</span></span>
            <span class="subj ellipsis">{m.subject}</span>
            {#if m.snippet}<span class="snip ellipsis">{m.snippet}</span>{/if}
          </span>
        </button>
      {/each}
      {#each sel.rest as m (m.account + m.uid)}
        <button class="row" onclick={() => read(m)}>
          {@render avatar(m, 28)}
          <span class="body">
            <span class="top"><b class="ellipsis">{m.fromName}</b><span class="time">{mailTime(m.date)}</span></span>
            <span class="subj ellipsis dim">{m.subject}</span>
          </span>
        </button>
      {/each}
      {#if !sel.important.length && !sel.rest.length}
        <p class="empty">{(t.categories[cat] ?? 0) ? 'Older mail in this category is in Gmail.' : 'All caught up here ✨'}</p>
      {:else if (t.categories[cat] ?? 0) > sel.important.length + sel.rest.length}
        <p class="more">Showing the newest {sel.important.length + sel.rest.length} of {t.categories[cat]}.</p>
      {/if}
    {/if}
  </div>

  {#if shown.length}
    <p class="foot" class:offline>{offline ? 'Offline · last checked' : 'Checked'} {relTime(checkedAt)} · stays unread in Gmail</p>
  {/if}
{/if}

<style>
  /* Summary card: the accent, deepened toward violet. */
  .hero {
    position: relative; overflow: hidden; padding: 14px 16px; border-radius: var(--r-outer, 16px); margin-bottom: var(--s-2);
    color: var(--on-accent, #fff);
    background: linear-gradient(135deg, var(--accent), color-mix(in oklab, var(--accent), #8b5cf6 55%));
    box-shadow: 0 6px 20px color-mix(in srgb, var(--accent) 30%, transparent);
  }
  .hero::after { content: ''; position: absolute; right: -36px; top: -36px; width: 130px; height: 130px; border-radius: 50%; background: rgba(255, 255, 255, 0.13); pointer-events: none; }
  .hero.quiet { background: var(--bg-2); color: var(--fg); box-shadow: none; }
  .hero.quiet::after { display: none; }
  .n { font-size: 32px; font-weight: 700; line-height: 1; font-variant-numeric: tabular-nums; }
  .s { font-size: 12px; opacity: 0.92; margin-top: 4px; }
  .who { font-size: 11px; opacity: 0.8; margin-top: 8px; }
  .accs { position: relative; z-index: 1; display: flex; gap: 6px; margin-top: 10px; flex-wrap: wrap; }
  .accs button { display: inline-flex; align-items: center; gap: 5px; font-size: 11px; padding: 3px 10px; border-radius: 999px; background: rgba(255, 255, 255, 0.2); color: inherit; font-variant-numeric: tabular-nums; }
  .accs button:hover { background: rgba(255, 255, 255, 0.3); }
  .accs button.on { background: #fff; color: #1c1f2a; font-weight: 600; }
  .hero.quiet .accs button { background: var(--pill); }
  .hero.quiet .accs button.on { background: var(--accent); color: var(--on-accent); }
  .acc-dot { display: inline-block; width: 7px; height: 7px; border-radius: 50%; vertical-align: middle; margin-right: 3px; }
  .accs .acc-dot { margin: 0; box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.7); }

  .warn { display: flex; align-items: center; gap: var(--s-2); justify-content: space-between; font-size: 12px; padding: 6px 10px; margin-bottom: var(--s-2); border-radius: var(--r-inner); background: color-mix(in srgb, #ff9f0a 16%, transparent); }
  .warn button { flex: none; font-size: 12px; font-weight: 600; color: var(--accent); }

  .chips { display: flex; gap: 6px; margin-bottom: var(--s-2); overflow-x: auto; scrollbar-width: none; }
  .chips button { flex: none; display: inline-flex; align-items: center; gap: 4px; font-size: 11.5px; padding: 5px 11px; border-radius: 999px; background: var(--bg-2); color: var(--fg-dim); }
  .chips button:hover { background: var(--pill-hover); color: var(--fg); }
  .chips button.on { background: var(--fg); color: var(--bg-0, #fff); }
  :global([data-theme='light']) .chips button.on { color: #fff; }
  :global([data-theme='dark']) .chips button.on { color: #111; }
  .chips b { font-weight: 600; font-variant-numeric: tabular-nums; opacity: 0.85; }

  .list { display: flex; flex-direction: column; gap: 2px; max-height: 300px; overflow-y: auto; margin: 0 calc(-1 * var(--s-1)); padding: 2px var(--s-1); }
  .imp, .row { display: flex; gap: 10px; align-items: flex-start; width: 100%; text-align: left; border-radius: var(--r-inner); transition: background 0.15s var(--ease), transform 0.2s var(--spring); }
  .imp { padding: 10px 12px; margin-bottom: 4px; background: var(--pill); border: 1px solid var(--line-soft); border-left: 3px solid #f59e0b; box-shadow: 0 2px 10px rgba(0, 0, 0, 0.06); }
  .row { padding: 7px 8px; align-items: center; }
  .imp:hover, .row:hover { background: var(--pill-hover); }
  .imp:active, .row:active { transform: scale(0.985); }
  .imp:focus-visible, .row:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .av { position: relative; flex: none; border-radius: 50%; display: grid; place-items: center; font-weight: 600; color: #fff; background: hsl(var(--h, 210) 55% 50%); }
  .av i { position: absolute; right: -2px; bottom: -2px; width: 11px; height: 11px; border-radius: 50%; border: 2px solid var(--bg-1, #f2f4fa); }
  .body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .top { display: flex; justify-content: space-between; gap: var(--s-2); font-size: 13px; }
  .time { flex: none; font-size: 11px; color: var(--fg-3); font-variant-numeric: tabular-nums; }
  .subj { font-size: 12.5px; }
  .snip { font-size: 12px; color: var(--fg-dim); }
  .empty, .more { margin: var(--s-4) 0; text-align: center; font-size: 12px; color: var(--fg-dim); }
  .more { margin: var(--s-2) 0; color: var(--fg-3); }
  .foot { margin: var(--s-2) 0 0; font-size: 11px; color: var(--fg-3); }
  .foot.offline { color: #ff9f0a; }
  .err { font-size: 12px; color: #ff453a; }
  .spin :global(svg) { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .back { display: inline-flex; align-items: center; gap: 4px; font-size: 12px; color: var(--fg-dim); margin-bottom: var(--s-2); padding: 2px 6px 2px 2px; border-radius: var(--r-inner); }
  .back:hover { color: var(--fg); background: var(--pill-hover); }
  .msg h4 { display: flex; align-items: center; gap: 6px; margin: 0 0 var(--s-1); font-size: 15px; font-weight: 600; line-height: 1.3; }
  .msg h4 :global(.star) { flex: none; color: #f59e0b; fill: #f59e0b; }
  .meta { display: flex; flex-direction: column; gap: 1px; font-size: 12px; margin-bottom: var(--s-3); }
  .text { white-space: pre-wrap; overflow-wrap: anywhere; font-size: 13px; line-height: 1.55; padding: var(--s-3); border-radius: var(--r-inner); background: var(--bg-2); max-height: 330px; overflow-y: auto; user-select: text; }
  .note { font-size: 11px; color: var(--fg-3); margin: var(--s-2) 0 0; }
  .ghost-btn { display: inline-flex; align-items: center; gap: 6px; margin-top: var(--s-3); padding: 7px 12px; border-radius: var(--r-inner); background: var(--pill); border: 1px solid var(--line-soft); font-size: 12px; }
  .ghost-btn:hover { background: var(--pill-hover); }

  .skeleton i { display: block; height: 10px; width: 80%; margin: 6px 0; border-radius: 4px; background: var(--track); animation: pulse 1.2s ease-in-out infinite; }
  .row.skeleton .av { width: 28px; height: 28px; background: var(--track); }
  @keyframes pulse { 50% { opacity: 0.45; } }
</style>
