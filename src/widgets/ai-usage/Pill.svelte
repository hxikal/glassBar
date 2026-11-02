<script lang="ts">
  // One row per Claude limit: Session (5-hour), Weekly, then per-model weekly (e.g. Fable).
  // Shows % left or % used per the widget's settings; the bar's colour always warms up
  // as the limit runs out.
  import { tickingNow } from '../../lib/now.svelte';
  import { loadTone } from '../../lib/side';
  import type { PillProps } from '../types';
  import type { AiUsageSettings } from './index';
  import { fmtLeft, pillRows, shownPct, visibleRows } from './format';
  import { liveLimits } from './limits.svelte';
  import ClaudeMark from './ClaudeMark.svelte';

  let { size, settings, vertical }: PillProps<AiUsageSettings> = $props();
  const pct = (used: number) => shownPct(used, settings.show);
  const word = $derived(settings.show === 'used' ? 'used' : 'left');
  const lim = liveLimits();
  const clock = tickingNow(() => true, 60_000);
  const all = $derived(lim.l ? pillRows(lim.l.windows) : []);
  // The rows ticked in settings; at small size only the first of them.
  const rows = $derived(visibleRows(all, settings.hidden ?? []).slice(0, size === 'small' ? 1 : undefined));
  const resetIn = (r: number | null) => (r ? fmtLeft(r - clock.now) : '–');
  const tip = $derived(lim.l
    ? all.map((r) => r.w ? `${r.label}: ${pct(r.w.usedPct)}% ${word}, resets in ${resetIn(r.w.resetsAt)}` : `${r.label}: no limit reported right now`).join('\n')
      + (lim.l.cached ? '\n(offline: last known)' : '')
    : lim.error || 'Loading Claude usage…');
</script>

{#if vertical}
  <div class="side-stack" title={tip}>
    <div class="mark"><ClaudeMark size={26} /></div>
    {#each rows as r (r.label)}
      <div class="side">
        <span class="side-k">{r.label}</span><span class="side-v">{r.w ? pct(r.w.usedPct) : '–'}{#if r.w}<small>%</small>{/if}</span>
        <span class="side-meter"><i style:width="{r.w ? pct(r.w.usedPct) : 0}%" style:--c={r.w ? loadTone(r.w.usedPct) : 'transparent'}></i></span>
      </div>
    {:else}
      <div class="side"><span class="side-k">AI</span><span class="side-v">{lim.error ? '!' : '–'}</span></div>
    {/each}
  </div>
{:else}
  <div class="pill" title={tip}>
    <ClaudeMark size={26} />
    {#if rows.length}
      <div class="rows">
        {#each rows as r (r.label)}
          <span class="lbl">{r.label}</span>
          <span class="meter"><i style:width="{r.w ? pct(r.w.usedPct) : 0}%" style:background={r.w ? loadTone(r.w.usedPct) : 'transparent'}></i></span>
          <b>{r.w ? `${pct(r.w.usedPct)}%` : '–'}</b>
        {/each}
      </div>
    {:else}
      <span class="dim">AI</span><b class:err={!!lim.error}>{lim.error ? '!' : '–'}</b>
    {/if}
    {#if lim.l?.cached}<span class="dot"></span>{/if}
  </div>
{/if}

<style>
  /* label | bar | %, one line per limit, packed to fit the dock's height. */
  .rows { display: grid; grid-template-columns: auto 80px auto; align-items: center; column-gap: 6px; row-gap: 1px; line-height: 1.1; }
  .lbl { font-size: 10px; opacity: 0.7; }
  b { font-variant-numeric: tabular-nums; font-size: 10px; text-align: right; min-width: 3ch; }
  .dim { font-size: 11px; }
  .meter { display: block; height: 5px; border-radius: 3px; background: color-mix(in srgb, currentColor 18%, transparent); overflow: hidden; }
  .meter i { display: block; height: 100%; border-radius: 3px; transition: width 0.4s; }
  .err { color: #ff453a; }
  .mark { display: grid; place-items: center; padding: 2px 0; }
  .dot { width: 6px; height: 6px; border-radius: calc(3px * var(--rs)); background: #ff9f0a; }
</style>
