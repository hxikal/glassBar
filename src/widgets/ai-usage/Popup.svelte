<script lang="ts">
  import { RefreshCw } from '@lucide/svelte';
  import PanelHeader from '../../lib/PanelHeader.svelte';
  import { ipc, type AiDay } from '../../lib/ipc';
  import { loadTone } from '../../lib/side';
  import { relTime } from '../../lib/time';
  import type { PopupProps } from '../types';
  import type { AiUsageSettings } from './index';
  import { barAt, fmtLeft, fmtTokens, modelName, shownPct, tipLeft } from './format';
  import { updateWidgetSettings } from '../../lib/config.svelte';
  import { liveLimits } from './limits.svelte';

  let { instance, settings }: PopupProps<AiUsageSettings> = $props();
  const lim = liveLimits();

  // Daily tokens from local Claude Code logs, read each time the panel opens.
  let days = $state<AiDay[]>([]);
  let dayErr = $state('');
  let loadingDays = $state(true);
  const loadDays = () => {
    loadingDays = true;
    ipc.aiDaily(settings.days).then((d) => { days = d; dayErr = ''; }).catch((e) => (dayErr = String(e))).finally(() => (loadingDays = false));
  };
  $effect(() => { settings.days; loadDays(); });
  const refresh = () => { lim.refresh(); loadDays(); };

  const TYPES = [
    { k: 'output', name: 'Output', c: '#0a84ff' },
    { k: 'input', name: 'Input', c: '#64d2ff' },
    { k: 'cacheWrite', name: 'Cache write', c: '#bf5af2' },
    { k: 'cacheRead', name: 'Cache read', c: '#8e8e93' },
  ] as const;
  type TypeKey = (typeof TYPES)[number]['k'];
  // Cache reads dwarf the rest, so they start hidden; click the legend to toggle a type.
  let hidden = $state<Set<TypeKey>>(new Set(['cacheRead']));
  const toggle = (k: TypeKey) => { const h = new Set(hidden); h.has(k) ? h.delete(k) : h.add(k); hidden = h; };
  const on = $derived(TYPES.filter((t) => !hidden.has(t.k)));
  const sum = (d: AiDay) => on.reduce((a, t) => a + d[t.k], 0);
  const max = $derived(Math.max(1, ...days.map(sum)));
  const total = $derived(days.reduce((a, d) => a + sum(d), 0));

  const W = 380, H = 140, GAP = 2;
  const bw = $derived(days.length ? W / days.length : W);
  // The hovered day comes from the pointer's x, so moving across the chart never skips or
  // flickers between bars, and the tooltip floats so nothing below it moves.
  let hover = $state<number | null>(null);
  const pick = $derived(hover == null ? null : days[hover]);
  const onMove = (e: PointerEvent) => {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    hover = barAt(e.clientX - r.left, r.width, days.length);
  };
  const fmtDate = (s: string, opts: Intl.DateTimeFormatOptions) => new Intl.DateTimeFormat(undefined, opts).format(new Date(s + 'T12:00:00'));
  const when = (ms: number) => new Intl.DateTimeFormat(undefined, { weekday: 'short', hour: 'numeric', minute: '2-digit' }).format(ms);
</script>

<PanelHeader title="AI Usage">
  <div class="seg" role="group" aria-label="Show percent">
    {#each ['left', 'used'] as const as m (m)}
      <button class:sel={settings.show === m} aria-pressed={settings.show === m} onclick={() => updateWidgetSettings(instance, { show: m })}>{m === 'left' ? 'Left' : 'Used'}</button>
    {/each}
  </div>
  <button class="icon-btn" class:spin={lim.loading || loadingDays} onclick={refresh} aria-label="Refresh" title="Refresh"><RefreshCw size={15} /></button>
</PanelHeader>

{#if lim.l}
  <div class="limits">
    {#each lim.l.windows as w (w.key)}
      <div class="card lim">
        <div class="row"><span class="dim small">{w.name}</span><b>{shownPct(w.usedPct, settings.show)}% {settings.show}</b></div>
        <span class="meter"><i style:width="{shownPct(w.usedPct, settings.show)}%" style:background={loadTone(w.usedPct)}></i></span>
        <div class="dim small">{w.resetsAt ? `Resets in ${fmtLeft(w.resetsAt - Date.now())} · ${when(w.resetsAt)}` : 'No reset scheduled'}</div>
      </div>
    {/each}
  </div>
  {#if lim.l.breakdown.length}
    <div class="share small" title="Share of this week's usage">
      {#each lim.l.breakdown as b (b.name)}<span><span class="dim">{b.name}</span> {Math.round(b.pct)}%</span>{/each}
    </div>
  {/if}
  <p class="small meta" class:offline={lim.l.cached}>
    {#if !lim.l.windows.some((w) => w.key === 'five_hour')}No 5-hour limit reported right now ·{/if}
    {lim.l.cached ? 'Offline · last known' : 'Claude'}{lim.l.plan ? ` ${lim.l.plan}` : ''} plan · checked {relTime(lim.l.fetchedAt, Date.now())}
  </p>
{:else if lim.error}
  <p class="err small">{lim.error}</p>
{/if}

<div class="card chart">
  <div class="row"><span class="dim small">Claude Code tokens · last {settings.days} days</span><b>{fmtTokens(total)}</b></div>
  {#if dayErr}
    <p class="err small">Couldn't read usage logs: {dayErr}</p>
  {:else if loadingDays && !days.length}
    <p class="dim small">Reading Claude Code logs…</p>
  {:else}
    <div class="plot" role="presentation" onpointermove={onMove} onpointerleave={() => (hover = null)}>
      <svg viewBox="0 0 {W} {H}" width="100%" role="img" aria-label="Daily token usage">
        {#each days as d, i (d.date)}
          <g class="bar" class:dimmed={hover != null && hover !== i}>
            {#each on as t, j (t.k)}
              {@const below = on.slice(0, j).reduce((a, u) => a + d[u.k], 0)}
              {@const h = (d[t.k] / max) * H}
              {#if h > 0}<rect x={i * bw + GAP / 2} y={H - ((below / max) * H) - h} width={Math.max(1, bw - GAP)} height={h} fill={t.c} />{/if}
            {/each}
          </g>
        {/each}
      </svg>
      {#if pick && hover != null}
        <div class="tip" style:left="{tipLeft(hover, days.length)}%" class:right={hover >= days.length / 2}>
          <div class="row"><b>{fmtDate(pick.date, { weekday: 'short', day: 'numeric', month: 'short' })}</b><b>{fmtTokens(sum(pick))}</b></div>
          {#each TYPES as t (t.k)}
            <div class="row" class:dim={hidden.has(t.k)}><span><i class="sw" style:background={t.c}></i>{t.name}</span><span>{pick[t.k].toLocaleString()}</span></div>
          {/each}
          {#each Object.entries(pick.byModel).sort((a, b) => b[1] - a[1]) as [m, n] (m)}
            <div class="row dim"><span>{modelName(m)}</span><span>{fmtTokens(n)}</span></div>
          {:else}
            <div class="dim">No usage</div>
          {/each}
        </div>
      {/if}
    </div>
    {#if days.length}
      <div class="axis dim small"><span>{fmtDate(days[0].date, { day: 'numeric', month: 'short' })}</span><span>peak {fmtTokens(max)}</span><span>Today</span></div>
    {/if}
    <div class="legend">
      {#each TYPES as t (t.k)}
        <button class="chip" class:off={hidden.has(t.k)} onclick={() => toggle(t.k)}><i style:background={t.c}></i>{t.name}</button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .limits { display: grid; grid-template-columns: repeat(auto-fit, minmax(110px, 1fr)); gap: 8px; }
  .lim { padding: 10px; display: flex; flex-direction: column; gap: 6px; }
  .seg { display: inline-flex; padding: 2px; gap: 2px; border-radius: calc(10px * var(--rs)); background: var(--pill); }
  .seg button { padding: 3px 9px; font-size: 11px; border-radius: calc(8px * var(--rs)); }
  .seg button.sel { background: var(--accent); color: var(--on-accent); }
  .share { display: flex; flex-wrap: wrap; gap: 4px 12px; margin-top: 8px; }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
  b { font-variant-numeric: tabular-nums; }
  .meter { display: block; height: 6px; border-radius: 3px; background: color-mix(in srgb, currentColor 18%, transparent); overflow: hidden; }
  .meter i { display: block; height: 100%; border-radius: 3px; }
  .chart { margin-top: 10px; padding: 10px; display: flex; flex-direction: column; gap: 6px; }
  .plot { position: relative; cursor: crosshair; }
  svg { display: block; }
  .bar { transition: opacity 0.12s; }
  .bar.dimmed { opacity: 0.4; }
  /* Sits beside the hovered bar (left of it on the right half), never over the pointer. */
  .tip {
    position: absolute; top: 0; z-index: 2; pointer-events: none; white-space: nowrap; max-height: 100%; overflow: hidden;
    transform: translateX(8px); padding: 6px 9px; display: flex; flex-direction: column; gap: 1px; line-height: 1.35;
    font-size: 11px; border-radius: calc(10px * var(--rs)); background: var(--menu-bg);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3); backdrop-filter: blur(12px);
  }
  .tip.right { transform: translateX(calc(-100% - 8px)); }
  .tip .row { gap: 14px; }
  .axis { display: flex; justify-content: space-between; }
  .legend { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { display: inline-flex; align-items: center; gap: 5px; padding: 4px 9px; border-radius: calc(12px * var(--rs)); background: var(--pill); font-size: 11px; }
  .chip i, .sw { display: inline-block; width: 8px; height: 8px; border-radius: 2px; margin-right: 5px; }
  .chip i { margin-right: 0; }
  .chip.off { opacity: 0.45; }
  .small { font-size: 11px; }
  .meta { color: var(--fg-3); margin: 6px 0 0; }
  .meta.offline { color: #ff9f0a; }
  .err { color: #ff453a; }
</style>
