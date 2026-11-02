<script lang="ts" module>
  import { ipc, type InstalledApp } from '../../lib/ipc';
  // Installed apps load once per popup window, on first use.
  let appsP: Promise<InstalledApp[]> | null = null;
  const apps = () => (appsP ??= ipc.appsInstalled().catch(() => []));
  const HISTORY = 'glassbar.search.history';
</script>

<script lang="ts">
  // GlassBar search: maths, units and currency answered as you type, matching apps, and
  // "Search Windows" / "Search the web" for anything else. ↑ ↓ move, Enter acts, Esc closes.
  import { ArrowRightLeft, Calculator, Check, Copy, Globe, History, Ruler, Search } from '@lucide/svelte';
  import { profile } from '../../lib/config.svelte';
  import FileIcon from '../downloads/FileIcon.svelte';
  import { iconOf } from '../folders/appIcons.svelte';
  import { formatNumber, parse, type Answer } from './query';
  import { labelOf } from './units';

  let { close }: { close: () => void } = $props();
  let q = $state('');
  let sel = $state(0);
  let copied = $state<string | null>(null);
  let input = $state<HTMLInputElement>();
  let all = $state<InstalledApp[]>([]);
  $effect(() => { input?.focus(); apps().then((a) => (all = a)); });

  const loadHistory = (): string[] => { try { return JSON.parse(localStorage.getItem(HISTORY) ?? '[]'); } catch { return []; } };
  let history = $state<string[]>(loadHistory());
  const remember = (expr: string) => {
    history = [expr, ...history.filter((h) => h !== expr)].slice(0, 5);
    try { localStorage.setItem(HISTORY, JSON.stringify(history)); } catch { /* private window: fine */ }
  };

  const answer = $derived<Answer | null>(parse(q));
  // Currency: the rate for `from`, fetched (and cached an hour) by the Currency widget's code.
  const defaultTo = $derived(((profile().widgets.find((w) => w.id === 'currency')?.settings as { to?: string } | undefined)?.to ?? 'MYR').toUpperCase());
  let fx = $state<{ key: string; value: number | null; note: string } | null>(null);
  $effect(() => {
    const a = answer;
    if (a?.kind !== 'fx') { fx = null; return; }
    const to = a.to ?? (a.from === defaultTo ? 'USD' : defaultTo);
    const key = `${a.amount}${a.from}${to}`;
    fx = { key, value: null, note: 'Getting the rate…' };
    ipc.fxRates(a.from).then((r) => {
      if (fx?.key !== key) return;
      const rate = r.rates[to];
      const mins = Math.max(0, Math.round((Date.now() - r.fetchedAt) / 60_000));
      fx = rate ? { key, value: a.amount * rate, note: `${to} · ${r.source}${r.cached ? ` · ${mins} min ago` : ''} · indicative rate` } : { key, value: null, note: `No rate for ${to}` };
    }).catch((e) => { if (fx?.key === key) fx = { key, value: null, note: String(e) }; });
  });

  type Row = { id: string; kind: 'answer' | 'app' | 'windows' | 'web' | 'history'; title: string; sub?: string; copy?: string; app?: InstalledApp; expr?: string };
  const words = $derived(q.toLowerCase().split(/\s+/).filter(Boolean));
  const matches = $derived(!words.length || answer ? [] : all.filter((a) => words.every((w) => a.name.toLowerCase().includes(w)))
    .sort((a, b) => Number(!b.name.toLowerCase().startsWith(words[0])) - Number(!a.name.toLowerCase().startsWith(words[0])) || a.name.length - b.name.length).slice(0, 5));

  const rows = $derived.by<Row[]>(() => {
    const out: Row[] = [];
    const a = answer;
    if (a?.kind === 'math') out.push({ id: 'ans', kind: 'answer', title: `= ${formatNumber(a.value)}`, sub: q.trim(), copy: String(Number(a.value.toPrecision(12))) });
    if (a?.kind === 'unit') out.push({ id: 'ans', kind: 'answer', title: `${formatNumber(a.value, 4)} ${labelOf(a.to)}`, sub: `${formatNumber(a.amount)} ${labelOf(a.from)}`, copy: String(Number(a.value.toPrecision(12))) });
    if (a?.kind === 'units') a.list.forEach((x, i) => out.push({ id: `u${i}`, kind: 'answer', title: `${formatNumber(x.value, 4)} ${labelOf(x.to)}`, sub: `${formatNumber(a.amount)} ${labelOf(a.from)}`, copy: String(Number(x.value.toPrecision(12))) }));
    if (a?.kind === 'fx') out.push({ id: 'ans', kind: 'answer', title: fx?.value != null ? formatNumber(Math.round(fx.value * 100) / 100) : '…', sub: `${formatNumber(a.amount)} ${a.from} · ${fx?.note ?? ''}`, copy: fx?.value != null ? (Math.round(fx.value * 100) / 100).toFixed(2) : undefined });
    for (const app of matches) out.push({ id: `app:${app.path}${app.args}`, kind: 'app', title: app.name, app });
    if (!q.trim()) history.forEach((h, i) => out.push({ id: `h${i}`, kind: 'history', title: h, expr: h }));
    if (q.trim()) {
      out.push({ id: 'win', kind: 'windows', title: `Search Windows for “${q.trim()}”` });
      out.push({ id: 'web', kind: 'web', title: 'Search the web' });
    }
    return out;
  });
  $effect(() => { void q; sel = 0; });

  async function act(r: Row) {
    if (r.kind === 'answer') {
      if (!r.copy) return;
      await ipc.ocrCopy(r.copy).catch(() => {});
      copied = r.id;
      if (answer?.kind === 'math') remember(q.trim());
      setTimeout(() => { if (copied === r.id) copied = null; }, 1200);
    } else if (r.kind === 'app' && r.app) {
      close();
      ipc.appLaunch(r.app.path, r.app.args, r.app.aumid).catch(() => {});
    } else if (r.kind === 'windows') {
      const text = q.trim();
      close();
      ipc.windowsSearchText(text).catch(() => {});
    } else if (r.kind === 'web') {
      close();
      ipc.appLaunch(`https://www.google.com/search?q=${encodeURIComponent(q.trim())}`).catch(() => {});
    } else if (r.kind === 'history' && r.expr) {
      q = r.expr;
      input?.focus();
    }
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') { e.preventDefault(); sel = Math.min(rows.length - 1, sel + 1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); sel = Math.max(0, sel - 1); }
    else if (e.key === 'Enter') { e.preventDefault(); const r = rows[sel]; if (r) act(r); }
    else if (e.key === 'Tab' && rows[sel]?.kind === 'app') { e.preventDefault(); q = rows[sel].title; }
  }
  const ICON = { answer: Calculator, windows: Search, web: Globe, history: History } as const;
</script>

<div class="box">
  <Search size={18} class="lead" />
  <input bind:this={input} bind:value={q} onkeydown={key} placeholder="Calculate, convert, or find an app" spellcheck="false" autocomplete="off"
         aria-label="Search" aria-controls="results" aria-activedescendant={rows[sel] ? `r-${rows[sel].id}` : undefined} />
</div>

<div class="results" id="results" role="listbox" aria-label="Results">
  {#if !q.trim() && history.length}<h4 class="dim">Recent calculations</h4>{/if}
  {#each rows as r, i (r.id)}
    {#if r.kind === 'app' && (i === 0 || rows[i - 1].kind !== 'app')}<h4 class="dim">Apps</h4>{/if}
    {#if (r.kind === 'windows') && i > 0}<div class="sep"></div>{/if}
    <button id="r-{r.id}" class="row {r.kind}" class:sel={i === sel} role="option" aria-selected={i === sel}
            onpointerenter={() => (sel = i)} onclick={() => act(r)}>
      <span class="ic">
        {#if r.kind === 'app' && r.app?.aumid}
          <!-- Store apps: their registered icon (their .exe sits in a folder we can't read). -->
          {@const src = iconOf({ path: r.app.path, aumid: r.app.aumid })}
          {#if src}<img {src} alt="" width="20" height="20" draggable="false" />{:else}<FileIcon path={r.app.path} size={20} />{/if}
        {:else if r.kind === 'app' && r.app}<FileIcon path={r.app.path} size={20} />
        {:else if r.kind === 'answer' && answer?.kind === 'fx'}<ArrowRightLeft size={16} />
        {:else if r.kind === 'answer' && (answer?.kind === 'unit' || answer?.kind === 'units')}<Ruler size={16} />
        {:else}{@const I = ICON[r.kind as keyof typeof ICON]}<I size={16} />{/if}
      </span>
      <span class="col">
        <span class="title ellipsis">{r.title}</span>
        {#if r.sub}<span class="sub ellipsis">{r.sub}</span>{/if}
      </span>
      {#if r.kind === 'answer' && r.copy}
        <span class="hint">{#if copied === r.id}<Check size={13} strokeWidth={3} /> Copied{:else if i === sel}<Copy size={12} /> Enter{/if}</span>
      {:else if i === sel && r.kind !== 'history'}<span class="hint">Enter</span>{/if}
    </button>
  {:else}
    <p class="dim empty">Try “15% of 2400”, “5 km to mi”, “100 usd to myr” or an app’s name.</p>
  {/each}
</div>

<style>
  .box { position: relative; display: flex; align-items: center; }
  .box :global(.lead) { position: absolute; left: 12px; color: var(--fg-dim); pointer-events: none; }
  input { width: 100%; height: 44px; padding: 0 12px 0 40px; border-radius: var(--r-inner); background: var(--bg-2); border: 1px solid var(--border);
          color: var(--fg); font: inherit; font-size: 16px; }
  input:focus-visible { outline: 2px solid var(--accent); outline-offset: -1px; }
  .results { display: flex; flex-direction: column; gap: 2px; margin-top: 8px; }
  h4 { margin: 8px 2px 4px; font-size: 11.5px; font-weight: 500; }
  .sep { height: 1px; margin: 6px 4px; background: var(--border); }
  .row { display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 4px var(--s-3); border-radius: var(--r-inner); text-align: left; width: 100%; }
  .row.sel { background: var(--bg-2); box-shadow: inset 3px 0 0 var(--accent); }
  .ic { display: grid; place-items: center; width: 22px; flex: none; color: var(--fg-dim); }
  .col { flex: 1; min-width: 0; display: flex; flex-direction: column; line-height: 1.25; }
  .title { font-size: 13px; }
  .sub { font-size: 11.5px; color: var(--fg-3); font-variant-numeric: tabular-nums; }
  /* The answer is the hero: big, semibold, tabular. */
  .answer .title { font-size: 22px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .answer .ic { color: var(--accent); }
  .hint { display: inline-flex; align-items: center; gap: 4px; flex: none; font-size: 11px; color: var(--fg-dim); }
  .empty { text-align: center; font-size: 12px; padding: 14px 8px; margin: 0; }
</style>
