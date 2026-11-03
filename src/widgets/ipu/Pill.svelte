<script lang="ts">
  // The whole pill takes the status colour (blue → red), so air quality reads at a glance.
  import { Wind } from '@lucide/svelte';
  import type { PillProps } from '../types';
  import { BANDS, glassInkOf, inkOf } from './bands';
  import type { IpuSettings } from './index';
  import { label, liveIpu, selectionKey } from './ipu.svelte';

  let { settings, vertical }: PillProps<IpuSettings> = $props();
  const ipu = liveIpu(() => selectionKey(settings));
  const band = $derived(ipu.r ? BANDS[ipu.r.band] : null);
  const ink = $derived(ipu.r ? inkOf(ipu.r.band) : null);
  const glassInk = $derived(ipu.r ? glassInkOf(ipu.r.band) : null);
</script>

{#if vertical}
<!-- Left/right dock: one tile in the status colour. -->
<div class="side ipu" class:coloured={!!band} style:--c={band?.color} style:--ink={ink} style:--ink-glass={glassInk}
     title={ipu.r ? `${label(ipu.r)} ${ipu.r.value} · ${band?.name} · ${ipu.r.station?.name ?? ''}` : 'Air quality'}>
  {#if ipu.r}
    <span class="side-k ink">{ipu.r.kind === 'ipu' ? 'IPU' : 'AQI'}</span>
    <span class="side-v">{Math.round(ipu.r.value)}</span>
    <span class="side-c ink">{band?.short}</span>
  {:else}
    <Wind size={16} class="dim" />
  {/if}
</div>
{:else}
<div class="pill ipu" class:coloured={!!band} style:--c={band?.color} style:--ink={ink} style:--ink-glass={glassInk}
     title={ipu.r ? `${label(ipu.r)} ${ipu.r.value} · ${band?.name} · ${ipu.r.station?.name ?? ''}` : 'Air quality'}>
  {#if ipu.r}
    <Wind size={14} />
    <!-- Stacked: reading on top, status below. -->
    <span class="stack">
      <span class="row"><span class="v">{Math.round(ipu.r.value)}</span> <span class="lbl">{label(ipu.r)}</span></span>
      <span class="name">{band?.name}</span>
    </span>
  {:else}
    <Wind size={16} class="dim" />
  {/if}
</div>
{/if}

<style>
  .ipu.coloured {
    background: color-mix(in srgb, var(--c) 82%, transparent);
    border-color: color-mix(in srgb, var(--c) 100%, transparent);
    color: var(--ink);
  }
  .ipu.coloured:hover { background: var(--c); }
  .v { font-weight: 700; font-variant-numeric: tabular-nums; }
  .lbl { font-size: 10px; opacity: 0.8; }
  .ipu { gap: 6px; }
  .stack { display: flex; flex-direction: column; line-height: 1.15; }
  .row { white-space: nowrap; }
  .name { font-size: 10.5px; font-weight: 500; opacity: 0.9; white-space: nowrap; }
  .side.ipu.coloured .ink { color: inherit; opacity: 0.85; }
</style>
