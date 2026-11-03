<script lang="ts">
  // One bar per Claude limit ticked in settings: Session (5-hour), Weekly, then per-model weekly
  // (e.g. Fable). Stacked one per line, or side by side; "used" or "left" always written out.
  import { tickingNow } from '../../lib/now.svelte';
  import type { PillProps } from '../types';
  import type { AiUsageSettings } from './index';
  import { bothTip, fmtLeft, pctLabel, pillRows, visibleRows } from './format';
  import { liveLimits } from './limits.svelte';
  import ClaudeMark from './ClaudeMark.svelte';
  import Meter from './Meter.svelte';

  let { size, settings, vertical }: PillProps<AiUsageSettings> = $props();
  const lim = liveLimits();
  const clock = tickingNow(() => true, 60_000);
  const all = $derived(lim.l ? pillRows(lim.l.windows) : []);
  // The rows ticked in settings; at small size only the first of them.
  const rows = $derived(visibleRows(all, settings.hidden ?? []).slice(0, size === 'small' ? 1 : undefined));
  // With one bar both layouts look the same.
  const oneRow = $derived(settings.layout === 'row' && rows.length > 1);
  // Three or more stacked rows only fit the dock's height with thinner bars.
  const tight = $derived(!oneRow && rows.length >= 3);
  const resetIn = (r: number | null) => (r ? fmtLeft(r - clock.now) : '–');
  const tip = $derived(lim.l
    ? all.map((r) => r.w ? `${r.label}: ${bothTip(r.w.usedPct, resetIn(r.w.resetsAt))}` : `${r.label}: no limit reported right now`).join('\n')
      + (lim.l.cached ? '\n(offline: last known)' : '')
    : lim.error || 'Loading Claude usage…');
</script>

{#if vertical}
  <div class="side-stack" title={tip}>
    <div class="mark"><ClaudeMark size={26} /></div>
    {#each rows as r (r.label)}
      <div class="side">
        <span class="side-k">{r.label}</span>
        <span class="side-v">{r.w ? pctLabel(r.w.usedPct, settings.show).split(' ')[0] : '–'}</span>
        <span class="side-k">{settings.show}</span>
        <span class="vmeter"><Meter usedPct={r.w?.usedPct ?? null} show={settings.show} track={settings.track} h={6} /></span>
      </div>
    {:else}
      <div class="side"><span class="side-k">AI</span><span class="side-v">{lim.error ? '!' : '–'}</span></div>
    {/each}
  </div>
{:else}
  <div class="pill" title={tip}>
    <ClaudeMark size={26} />
    {#if rows.length}
      <div class="rows" class:one-row={oneRow} class:tight>
        {#each rows as r (r.label)}
          <div class="lim">
            <span class="lbl">{r.label}</span>
            <Meter usedPct={r.w?.usedPct ?? null} show={settings.show} track={settings.track} h={tight ? 6 : 10} />
            <b>{r.w ? pctLabel(r.w.usedPct, settings.show) : '–'}</b>
          </div>
        {/each}
      </div>
    {:else}
      <span class="dim">AI</span><b class:err={!!lim.error}>{lim.error ? '!' : '–'}</b>
    {/if}
    {#if lim.l?.cached}<span class="dot"></span>{/if}
  </div>
{/if}

<style>
  /* Stacked: label | bar | "42% used", one line per limit, packed to fit the dock's height. */
  .rows { display: grid; grid-template-columns: auto 96px auto; align-items: center; column-gap: 7px; row-gap: 8px; line-height: 1.1; }
  .tight { row-gap: 4px; }
  .lim { display: contents; }
  .lbl { font-size: 10px; opacity: 0.75; }
  b { font-variant-numeric: tabular-nums; font-size: 10px; font-weight: 600; text-align: right; white-space: nowrap; }
  /* One row: equal columns side by side, "Weekly 42% used" over each bar. */
  .one-row { display: flex; gap: 10px; }
  .one-row .lim { display: grid; grid-template-columns: auto 1fr; grid-template-rows: auto auto; column-gap: 4px; row-gap: 3px; width: 92px; align-items: baseline; }
  .one-row .lim > :global(.meter) { grid-column: 1 / -1; grid-row: 2; }
  .one-row b { text-align: left; }
  .dim { font-size: 11px; }
  .err { color: #ff453a; }
  .mark { display: grid; place-items: center; padding: 2px 0; }
  .vmeter { display: flex; width: calc(100% - 12px); margin-top: 3px; }
  .dot { width: 6px; height: 6px; border-radius: calc(3px * var(--rs)); background: #ff9f0a; }
</style>
