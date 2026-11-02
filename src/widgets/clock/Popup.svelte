<script lang="ts">
  import { ArrowDown, ArrowUp, ChevronLeft, ChevronRight, Moon, Plus, Settings2, Sun, Sunrise, Sunset, X } from '@lucide/svelte';
  import PanelHeader from '../../lib/PanelHeader.svelte';
  import FormatOptions from './FormatOptions.svelte';
  import WeatherOptions from './WeatherOptions.svelte';
  import HolidayOptions from './HolidayOptions.svelte';
  import { holidaysFor } from './holidays.svelte';
  import { daysUntil, STATES, whenLabel, ymd } from './holidays';
  import Time from './ClockTime.svelte';
  import { date, timeText } from './prefs.svelte';
  import { dayState, type DayState } from './solar';
  import { useLocation } from '../../lib/location.svelte';
  import { updateWidgetSettings } from '../../lib/config.svelte';
  import { ipc, type Place } from '../../lib/ipc';
  import type { PopupProps } from '../types';
  import type { ClockSettings } from './index';
  import { ticker } from './time.svelte';
  import { cityOf, diffLabel, localOffset, offsetMinutes, zonesOf, type Zone } from './zones';

  let { instance, settings }: PopupProps<ClockSettings> = $props();
  // Every second only when seconds are shown somewhere in the panel; otherwise each minute.
  const clock = ticker(() => settings.showSeconds || settings.worldSeconds);
  let showFormat = $state(false);
  const today = $derived(clock.now);
  let month = $state(new Date(new Date().getFullYear(), new Date().getMonth(), 1));
  const zones = $derived(zonesOf(settings));
  const save = (z: Zone[]) => updateWidgetSettings(instance, { zones: z, secondTz: '' });

  // ---- world clocks ----
  let adding = $state(false);
  let query = $state('');
  let cityResults = $state<Place[]>([]);
  let timer: ReturnType<typeof setTimeout>;
  const allZones = Intl.supportedValuesOf('timeZone');
  // Matching time-zone ids (e.g. "tokyo", "london") plus real city search with each city's zone.
  const zoneResults = $derived(query.trim().length < 2 ? [] :
    allZones.filter((z) => z.toLowerCase().replace(/_/g, ' ').includes(query.trim().toLowerCase())).slice(0, 5));
  function search(q: string) {
    query = q;
    clearTimeout(timer);
    if (q.trim().length < 2) { cityResults = []; return; }
    timer = setTimeout(() => ipc.locationSearch(q).then((r) => (cityResults = r.filter((p) => p.tz))), 300);
  }
  function add(z: Zone) {
    if (!zones.some((x) => x.tz === z.tz && x.label === z.label)) save([...zones, z]);
    adding = false; query = ''; cityResults = [];
  }
  const move = (i: number, d: number) => { const z = [...zones]; [z[i], z[i + d]] = [z[i + d], z[i]]; save(z); };
  const day = (tz: string) => date(clock.now, settings, tz);
  // Day/night per city (real sunrise/sunset when the city has coordinates), on the clock's tick.
  const icons: Record<DayState, typeof Sun> = { day: Sun, night: Moon, sunrise: Sunrise, sunset: Sunset };
  // A city in your own time zone without its own coordinates uses the shared location.
  const here = useLocation();
  const pcTz = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const coordsOf = (z: Zone) => {
    if (z.lat != null && z.lon != null) return { lat: z.lat, lon: z.lon };
    const p = here.s?.place;
    return p && z.tz === (p.tz ?? pcTz) ? { lat: p.lat, lon: p.lon } : undefined;
  };
  function light(z: Zone) {
    const c = coordsOf(z);
    const info = dayState(clock.now.getTime(), z.tz, c);
    const name = { day: 'Day', night: 'Night', sunrise: 'Sunrise', sunset: 'Sunset' }[info.state];
    const next = info.state === 'day' || info.state === 'sunset'
      ? (info.nextSet ? ` · sunset at ${timeText(new Date(info.nextSet), settings, false, z.tz)}` : '')
      : (info.nextRise ? ` · sunrise at ${timeText(new Date(info.nextRise), settings, false, z.tz)}` : '');
    const how = !c ? ' (approximate: 6 AM–6 PM)' : z.lat == null ? ' (your location)' : '';
    return { ...info, icon: icons[info.state], tip: name + next + how };
  }

  // ---- public holidays ----
  const hol = holidaysFor(() => settings, () => clock.now);
  let allHolidays = $state(false);
  const yearAhead = $derived(ymd(new Date(today.getFullYear() + 1, today.getMonth(), today.getDate())));
  const holList = $derived(hol.list.filter((h) => h.date < yearAhead).slice(0, allHolidays ? undefined : 5));
  const holDay = (d: string) => new Date(d + 'T00:00').toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' });
  const holOn = (d: number) => d ? hol.list.find((h) => h.date === ymd(new Date(month.getFullYear(), month.getMonth(), d))) : undefined;

  // ---- calendar (Monday-first) ----
  const cells = $derived.by(() => {
    const lead = (month.getDay() + 6) % 7;
    const days = new Date(month.getFullYear(), month.getMonth() + 1, 0).getDate();
    return [...Array(lead).fill(0), ...Array.from({ length: days }, (_, i) => i + 1)];
  });
  const isToday = (d: number) => d === today.getDate() && month.getMonth() === today.getMonth() && month.getFullYear() === today.getFullYear();
  const shift = (n: number) => (month = new Date(month.getFullYear(), month.getMonth() + n, 1));
  const weekdays = Array.from({ length: 7 }, (_, i) => new Intl.DateTimeFormat(undefined, { weekday: 'narrow' }).format(new Date(2024, 0, 1 + i)));
</script>

<PanelHeader title="Clock">
  <button class="icon-btn" class:on={showFormat} onclick={() => (showFormat = !showFormat)} aria-label="Clock options" title="Clock options"><Settings2 size={16} /></button>
</PanelHeader>
<div class="hero">
  <div class="big"><Time now={clock.now} {settings} /></div>
  <div class="dim">{date(clock.now, settings)}</div>
</div>
{#if !showFormat}
<label class="wxsw dim">Weather on the clock
  <input type="checkbox" class="switch" checked={settings.showWeather} onchange={(e) => updateWidgetSettings(instance, { showWeather: e.currentTarget.checked })} />
</label>
{/if}
{#if showFormat}
  <div class="fmt"><FormatOptions {settings} save={(p) => updateWidgetSettings(instance, p)} /></div>
  <div class="fmt"><WeatherOptions {settings} save={(p) => updateWidgetSettings(instance, p)} /></div>
  <div class="fmt"><HolidayOptions {settings} save={(p) => updateWidgetSettings(instance, p)} /></div>
{/if}

<div class="world">
  <div class="wtitle">
    <span class="dim">World clocks</span>
    <button class="icon-btn add" onclick={() => (adding = !adding)} aria-label="Add a city"><Plus size={15} /></button>
  </div>
  {#if adding}
    <!-- svelte-ignore a11y_autofocus -->
    <input class="search" placeholder="Search a city or time zone…" value={query} oninput={(e) => search(e.currentTarget.value)} autofocus />
    <div class="results">
      {#each cityResults as p (p.name + p.tz)}
        <button onclick={() => add({ tz: p.tz!, label: p.name.split(',')[0], lat: p.lat, lon: p.lon })}>{p.name} <span class="dim">· {p.tz}</span></button>
      {/each}
      {#each zoneResults as z (z)}
        <button onclick={() => add({ tz: z, label: cityOf(z) })}>{cityOf(z)} <span class="dim">· {z}</span></button>
      {/each}
    </div>
  {/if}
  {#each zones as z, i (z.tz + z.label)}
    {@const l = settings.dayNight ? light(z) : null}
    <div class="zrow" class:night={l?.state === 'night'} class:dayrow={l?.state === 'day'} class:twi={l?.state === 'sunrise' || l?.state === 'sunset'} tabindex="-1">
      {#if l}<span class="dn" title={l.tip} aria-label={l.tip}><l.icon size={16} /></span>{/if}
      <div class="zcol">
        <b class="ellipsis">{z.label}</b>
        <span class="dim small">{day(z.tz)} · {diffLabel(localOffset(clock.now), offsetMinutes(z.tz, clock.now))}</span>
      </div>
      <span class="ztime"><Time now={clock.now} {settings} seconds={settings.worldSeconds} tz={z.tz} /></span>
      <div class="ztools">
        <button disabled={i === 0} onclick={() => move(i, -1)} aria-label="Move up"><ArrowUp size={12} /></button>
        <button disabled={i === zones.length - 1} onclick={() => move(i, 1)} aria-label="Move down"><ArrowDown size={12} /></button>
        <button onclick={() => save(zones.filter((_, j) => j !== i))} aria-label="Remove {z.label}"><X size={12} /></button>
      </div>
    </div>
  {:else}
    {#if !adding}<p class="dim small empty">Add cities with + to compare times.</p>{/if}
  {/each}
</div>

<div class="card cal">
  <div class="nav">
    <button class="icon-btn" onclick={() => shift(-1)} aria-label="Previous month"><ChevronLeft size={16} /></button>
    <button class="title" onclick={() => (month = new Date(today.getFullYear(), today.getMonth(), 1))}>
      {new Intl.DateTimeFormat(undefined, { month: 'long', year: 'numeric' }).format(month)}
    </button>
    <button class="icon-btn" onclick={() => shift(1)} aria-label="Next month"><ChevronRight size={16} /></button>
  </div>
  <div class="grid">
    {#each weekdays as w, i (i)}<span class="wd dim">{w}</span>{/each}
    {#each cells as d, i (i)}
      {@const h = holOn(d)}
      <span class="day" class:today={d && isToday(d)} class:holiday={!!h} title={h?.name}>{d || ''}</span>
    {/each}
  </div>
</div>

{#if settings.holidayList}
<div class="hols">
  <div class="wtitle">
    <span class="dim">Public holidays</span>
    <select class="hstate" value={settings.holidayState} title="Whose state holidays to include"
            onchange={(e) => updateWidgetSettings(instance, { holidayState: e.currentTarget.value })}>
      <option value="auto">{hol.state && settings.holidayState === 'auto' ? `${hol.state} (my location)` : 'My location'}</option>
      {#each STATES as s (s)}<option value={s}>{s}</option>{/each}
    </select>
    <span class="grow"></span>
    {#if hol.list.filter((h) => h.date < yearAhead).length > 5}
      <button class="more" onclick={() => (allHolidays = !allHolidays)}>{allHolidays ? 'Less' : 'All'}</button>
    {/if}
  </div>
  {#each holList as h (h.date + h.name)}
    {@const n = daysUntil(h.date, hol.today)}
    <div class="hrow" class:soon={n <= 7}>
      <span class="hdate">{holDay(h.date)}</span>
      <span class="ellipsis hn" title={h.states.length ? `${h.name} (${h.states.join(', ')})` : h.name}>{h.name}</span>
      <span class="dim small">{whenLabel(n)}</span>
    </div>
  {:else}
    <p class="dim small empty">No holidays loaded yet (needs the internet once a week).</p>
  {/each}
  {#if !hol.state}<p class="dim small empty">Only national holidays: pick your state above for state ones.</p>{/if}
</div>
{/if}

<style>
  .hero { padding: 0 4px var(--s-3); }
  .wxsw { display: flex; align-items: center; justify-content: space-between; font-size: 12px; margin: 0 4px 10px; }
  .icon-btn.on { color: var(--accent); }
  .fmt { margin: 0; }
  .dn { display: grid; place-items: center; width: 26px; height: 26px; border-radius: calc(13px * var(--rs)); flex: none; }
  .dayrow .dn { color: #ffb340; }
  .night .dn { color: #9fb4ff; }
  .twi .dn { color: #ff8a65; }
  /* Rows are plain inset surfaces; day/night shows only in the small icon. */
  .big { font-size: 30px; font-weight: 600; font-variant-numeric: tabular-nums; line-height: 1.1; }
  .world { margin-bottom: 10px; }
  .wtitle { display: flex; align-items: center; justify-content: space-between; font-size: 12px; padding: 0 4px; }
  .add { width: 26px; height: 26px; }
  .search { width: 100%; padding: 7px 10px; border-radius: calc(10px * var(--rs)); border: 1px solid var(--line-soft); background: var(--pill); margin: 4px 0; outline: none; }
  .results { display: flex; flex-direction: column; }
  .results button { text-align: left; padding: 5px 8px; border-radius: calc(8px * var(--rs)); font-size: 12px; }
  .results button:hover { background: var(--pill-hover); }
  .zrow { display: flex; align-items: center; gap: var(--s-2); min-height: 40px; padding: var(--s-1) var(--s-3); border-radius: var(--r-inner); background: var(--bg-2); border: 1px solid var(--border); }
  .zrow + .zrow { margin-top: var(--s-1); }
  .zrow:hover { background: var(--pill-hover); }
  .zcol { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .zcol b { font-weight: 500; }
  .ztime { font-size: 18px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .ztools { display: flex; opacity: 0; transition: opacity 0.2s; }
  .zrow:hover .ztools, .zrow:focus-within .ztools { opacity: 1; }
  .ztools button { width: 20px; height: 20px; display: grid; place-items: center; border-radius: 50%; }
  .ztools button:hover { background: var(--pill-hover); }
  .ztools button:disabled { opacity: 0.3; }
  .small { font-size: 11px; }
  .empty { padding: 4px 6px; margin: 0; }
  .cal { padding: 10px; }
  .nav { display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px; }
  .title { font-weight: 600; padding: 4px 10px; border-radius: calc(10px * var(--rs)); }
  .title:hover { background: var(--pill-hover); }
  .grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 2px; text-align: center; }
  .wd { font-size: 11px; padding: 3px 0; }
  .day { padding: 5px 0; border-radius: 50%; font-variant-numeric: tabular-nums; font-size: 12px; }
  .today { background: var(--accent); color: var(--on-accent); font-weight: 600; }
  .holiday:not(.today) { color: #ff9f43; font-weight: 600; box-shadow: inset 0 0 0 1px color-mix(in srgb, #ff9f43 45%, transparent); }
  .hols { margin-top: 10px; }
  .hstate { font-size: 11px; padding: 2px 6px; border-radius: calc(8px * var(--rs)); border: 1px solid var(--line-soft); background: var(--pill); color: inherit; max-width: 170px; }
  .grow { flex: 1; }
  .hols .wtitle { gap: 6px; }
  .more { font-size: 11px; padding: 2px 8px; border-radius: calc(8px * var(--rs)); color: var(--accent); }
  .more:hover { background: var(--pill-hover); }
  .hrow { display: flex; align-items: center; gap: var(--s-2); padding: 6px var(--s-3); border-radius: var(--r-inner); font-size: 12px; }
  .hrow:hover { background: var(--pill-hover); }
  .hdate { width: 78px; flex: none; font-variant-numeric: tabular-nums; }
  .hn { flex: 1; min-width: 0; }
  .soon .hdate { color: #ff9f43; font-weight: 600; }
</style>
