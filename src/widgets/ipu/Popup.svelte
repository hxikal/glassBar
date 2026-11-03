<script lang="ts">
  import { MapPin, RefreshCw } from '@lucide/svelte';
  import { ipc } from '../../lib/ipc';
  import type { PopupProps } from '../types';
  import { BANDS, bandOf, glassInkOf } from './bands';
  import type { IpuSettings } from './index';
  import { label, liveIpu, selectionKey } from './ipu.svelte';
  import { updateWidgetSettings } from '../../lib/config.svelte';
  import IpuLocation from './IpuLocation.svelte';
  import LocationTag from '../../lib/LocationTag.svelte';

  let { instance, settings }: PopupProps<IpuSettings> = $props();
  const ipu = liveIpu(() => selectionKey(settings));
  let browsing = $state(false);
  const where = $derived(ipu.r?.kind === 'aqi_us' ? (ipu.r.place?.name ?? '') : ipu.r?.station ? `Malaysia · ${ipu.r.station.state}` : '');
  const time = (ms: number) => new Intl.DateTimeFormat(undefined, { weekday: 'short', hour: '2-digit', minute: '2-digit' }).format(ms);
  const km = (d: number | null) => (d == null ? '' : d < 1 ? '<1 km' : `${Math.round(d)} km`);
  let refreshing = $state(false);
  async function refresh() {
    refreshing = true;
    try { ipu.r = (await ipc.ipuGet(true)) ?? ipu.r; } finally { refreshing = false; }
  }
</script>

<div class="where">
  <span class="dim small">{where || 'Air quality'}</span>
  <span class="acts">
    <button class="change" onclick={() => (browsing = !browsing)}>{browsing ? 'Done' : 'Change location'}</button>
    <button class="icon-btn" class:spin={refreshing} onclick={refresh} aria-label="Refresh" title="Refresh now"><RefreshCw size={14} /></button>
  </span>
</div>
{#if browsing}
  <div class="card browse">
    <IpuLocation {settings} save={(p) => updateWidgetSettings(instance, p)} stations={ipu.r?.stations ?? []} />
  </div>
{/if}
{#if ipu.r}
  {@const r = ipu.r}
  {@const b = BANDS[r.band]}
  <div class="hero" style:--c={b.color} style:--ink={glassInkOf(r.band)}>
    <span class="lbl">{label(r)}</span>
    <span class="big">{Math.round(r.value)}</span>
    <span class="band">{b.name}</span>
  </div>
  {#if r.station}
    <div class="row"><MapPin size={13} /> <b>{r.station.name}</b> <span class="dim">{km(r.distanceKm)}</span></div>
    <div class="dim small">Reading {time(r.station.at)} · main pollutant {r.station.pollutant}{r.offline ? ' · offline, cached' : ''}</div>
  {:else}
    <div class="dim small">{r.place?.name ?? ''}{r.offline ? ' · offline, cached' : ''}</div>
  {/if}
  {#if r.nearby.length}
    <h4 class="dim">Nearby stations</h4>
    {#each r.nearby as [s, d] (s.id)}
      <div class="near">
        <span class="dot" style:background={BANDS[bandOf(s.api ?? 0)].color}></span>
        <span class="ellipsis grow">{s.name}</span><span class="dim">{km(d)}</span><b>{Math.round(s.api ?? 0)}</b>
      </div>
    {/each}
  {/if}
  {#if settings.country !== 'other'}<LocationTag />{/if}
  <p class="src dim">Data: {r.kind === 'ipu' ? 'JAS/DOE APIMS, updated hourly' : 'Open-Meteo (US AQI scale, not IPU)'}</p>
{:else if ipu.loading}
  <p class="dim empty">Loading air quality…</p>
{:else}
  <p class="dim empty">No reading available. Check your internet connection or set a location.</p>
  {#if settings.country !== 'other'}<LocationTag />{/if}
{/if}

<style>
  .where { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }
  .change { font-size: 12px; color: var(--accent); }
  .browse { padding: 10px; margin-bottom: 10px; }
  .hero { display: flex; flex-direction: column; align-items: center; padding: 18px; border-radius: calc(20px * var(--rs)); margin-bottom: 12px;
          background: color-mix(in srgb, var(--c) 30%, var(--pill)); border: 1px solid color-mix(in srgb, var(--c) 60%, transparent); }
  /* Liquid Glass: the reading sits in tinted glass, like the dock pill. */
  :global(:root[data-preset='liquid']:not([data-solid='true'])) .hero {
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.28), rgba(255, 255, 255, 0) 55%), color-mix(in srgb, var(--c) 55%, transparent);
    border-color: color-mix(in srgb, var(--c) 70%, #fff);
    box-shadow: inset 1.5px 1.5px 0 -0.5px rgba(255, 255, 255, 0.75), inset 0 -10px 16px -10px rgba(0, 0, 0, 0.25),
      0 6px 20px color-mix(in srgb, var(--c) 35%, transparent);
    color: var(--ink);
  }
  .lbl { font-size: 12px; font-weight: 500; opacity: 0.8; }
  .big { font-size: 52px; font-weight: 700; line-height: 1.05; font-variant-numeric: tabular-nums; }
  .band { font-weight: 600; }
  .row { display: flex; align-items: center; gap: 6px; }
  .small { font-size: 12px; margin-top: 2px; }
  h4 { margin: 14px 2px 4px; font-size: 12px; font-weight: 500; }
  .near { display: flex; align-items: center; gap: 8px; padding: 6px 2px; border-bottom: 1px solid var(--line-soft); font-size: 12px; }
  .grow { flex: 1; }
  .dot { width: 9px; height: 9px; border-radius: 50%; flex: none; }
  .src { font-size: 11px; margin-top: 10px; }
  .empty { text-align: center; padding: 60px 10px; }
  .acts { display: inline-flex; align-items: center; gap: 4px; }
  .spin :global(svg) { animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
