<script lang="ts">
  import { ipc } from '../../lib/ipc';
  import type { PillProps } from '../types';
  import type { ClockSettings } from './index';
  import { date, time } from './prefs.svelte';
  import Time from './ClockTime.svelte';
  import { ticker } from './time.svelte';
  import { zonesOf } from './zones';
  import WeatherPart from './WeatherPart.svelte';
  import { PartyPopper } from '@lucide/svelte';
  import { holidaysFor } from './holidays.svelte';
  import { daysUntil, whenLabel } from './holidays';

  let { instance, size, settings, vertical }: PillProps<ClockSettings> = $props();
  // Ticks every second only while seconds are shown; otherwise on each minute boundary.
  const clock = ticker(() => settings.showSeconds);
  const first = $derived(zonesOf(settings)[0]);
  // The small size always shows the time only.
  const show = $derived(size === 'small' ? 'time' : settings.show);
  const day = $derived(date(clock.now, settings));
  // Left/right dock: time without seconds, AM/PM under it, then a short weekday + day.
  const sideTime = $derived(time(clock.now, settings, false));
  // Next public holiday, when it is close enough to show beside the clock.
  const hol = holidaysFor(() => settings, () => clock.now);
  const next = $derived.by(() => {
    const h = settings.holidays ? hol.list[0] : undefined;
    const days = h ? daysUntil(h.date, hol.today) : Infinity;
    return h && days <= Number(settings.holidayDays) ? { ...h, days } : null;
  });
  const sideDay = $derived(new Intl.DateTimeFormat(undefined, { weekday: 'short', day: 'numeric' }).format(clock.now));

  // "Popup" appearance: hovering the clock shows the big clock + world clocks, without a click.
  let hoverTimer: ReturnType<typeof setTimeout>;
  function enter(e: PointerEvent) {
    if (settings.appearance !== 'popup') return;
    const el = e.currentTarget as HTMLElement;
    hoverTimer = setTimeout(() => ipc.popupOpen(`__hover:${instance}`, el.getBoundingClientRect(), 340, 640, false), 400);
  }
  function leave() {
    clearTimeout(hoverTimer);
    if (settings.appearance === 'popup') ipc.popupLeave();
  }
</script>

{#if vertical}
<div class="side-stack" onpointerenter={enter} onpointerleave={leave} role="timer" title={day}>
  {#if settings.showWeather}<WeatherPart {instance} colorful={settings.weatherColorful} vertical />{/if}
  <div class="side">
    <span class="side-v">{sideTime.main}</span>
    {#if sideTime.ampm && settings.ampmSize !== 'hidden'}<span class="side-k">{sideTime.ampm}</span>{/if}
    <span class="side-c">{sideDay}</span>
  </div>
</div>
{:else}
<div class="clockwrap" class:withwx={settings.showWeather} class:withhol={!!next}>
{#if settings.showWeather}<WeatherPart {instance} colorful={settings.weatherColorful} vertical={false} />{/if}
<div class="pill clock {settings.appearance}" class:stacked={show === 'both' && settings.layout === 'stacked'}
     class:date-first={show === 'both' && settings.timeSide === 'right'}
     data-align={settings.stackAlign}
     onpointerenter={enter} onpointerleave={leave} role="timer" title={day}>
  {#if show !== 'date'}
    <span class="time" class:big={settings.appearance === 'prominent'}><Time now={clock.now} {settings} /></span>
  {/if}
  {#if show === 'both' && settings.layout === 'inline'}<span class="dot dim">·</span>{/if}
  {#if show !== 'time'}<span class="date" class:dim={show === 'both'}>{day}</span>{/if}
  {#if size === 'large' && first}
    <span class="dim zone">{first.label} <Time now={clock.now} {settings} seconds={settings.worldSeconds} tz={first.tz} /></span>
  {/if}
</div>
{#if next}
  <span class="hol" title="{next.name} · {new Date(next.date).toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long' })} · {whenLabel(next.days)}">
    <PartyPopper size={14} />
    {#if size !== 'small'}<span class="hname">{next.name}</span>{/if}
    <b>{next.days === 0 ? 'Today' : next.days === 1 ? 'Tmrw' : `${next.days}d`}</b>
  </span>
{/if}
</div>
{/if}

<style>
  /* Weather (optional) sits inside the same pill shape, on the left. */
  .clockwrap { display: flex; align-items: stretch; gap: 2px; height: 100%; }
  .withwx:not(.vertical) > .clock { border-top-left-radius: calc(4px * var(--rs)); border-bottom-left-radius: calc(4px * var(--rs)); }
  /* Next holiday (optional) on the right, the same pill shape continued. */
  .withhol > .clock { border-top-right-radius: calc(4px * var(--rs)); border-bottom-right-radius: calc(4px * var(--rs)); }
  .hol {
    display: flex; align-items: center; gap: 5px; height: 100%; padding: 0 10px; white-space: nowrap;
    border-radius: 4px var(--radius-pill) var(--radius-pill) 4px;
    background: color-mix(in srgb, #ff9f43 22%, var(--pill)); font-size: 12px;
  }
  .hol:hover { background: color-mix(in srgb, #ff9f43 32%, var(--pill)); }
  .hol :global(svg) { color: #ff9f43; flex: none; }
  .hname { max-width: 130px; overflow: hidden; text-overflow: ellipsis; }
  .hol b { font-weight: 600; font-variant-numeric: tabular-nums; }
  .time { font-weight: 600; }
  .date { white-space: nowrap; font-variant-numeric: tabular-nums; }
  .dot { margin: 0 -2px; }
  /* date first: reverse the order (left/right, or top/bottom when stacked) */
  .date-first .time { order: 3; }
  .date-first .dot { order: 2; }
  .date-first .date { order: 1; }
  .date-first .zone { order: 4; }
  .zone { font-size: 12px; }
  /* stacked: time above the date */
  .stacked { flex-direction: column; justify-content: center; gap: 0; line-height: 1.15; }
  .stacked .date { font-size: 11px; }
  .stacked[data-align='left'] { align-items: flex-start; text-align: left; }
  .stacked[data-align='center'] { align-items: center; text-align: center; }
  .stacked[data-align='right'] { align-items: flex-end; text-align: right; }
  /* prominent: larger, bolder time on an accent-tinted panel, still within the bar height */
  .prominent {
    background: color-mix(in srgb, var(--accent) 30%, var(--pill));
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
    gap: 10px; padding: 0 14px;
  }
  .prominent:hover { background: color-mix(in srgb, var(--accent) 42%, var(--pill)); }
  .big { font-size: 19px; font-weight: 700; letter-spacing: 0.3px; line-height: 1; }
  .prominent .date { font-size: 12px; font-weight: 500; opacity: 0.85; }
  .prominent.stacked { gap: 2px; }
  .prominent.stacked .big { font-size: 16px; }
</style>
