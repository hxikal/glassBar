<script lang="ts">
  import { ArrowDown, ArrowUp, ChevronLeft, ChevronRight, Moon, Plus, Settings2, Sun, Sunrise, Sunset, X } from '@lucide/svelte';
  import PanelHeader from '../../lib/PanelHeader.svelte';
  import FormatOptions from './FormatOptions.svelte';
  import WeatherOptions from './WeatherOptions.svelte';
  import HolidayOptions from './HolidayOptions.svelte';
  import { holidaysFor } from './holidays.svelte';
  import { daysUntil, STATES, whenLabel, ymd } from './holidays';
  import { hijriDay, isoWeek, isWeekend, itemsByDay, monthCells } from './calendar';
  import { hijriLabel, hijriOf } from '../solat/hijri';
  import type { CountdownSettings } from '../countdown/index';
  import { live } from '../../lib/live.svelte';
  import Time from './ClockTime.svelte';
  import { date, timeText } from './prefs.svelte';
  import { dayState, sunTimes, type DayState } from './solar';
  import { useLocation } from '../../lib/location.svelte';
  import { profile, updateWidgetSettings } from '../../lib/config.svelte';
  import { ipc, type Place, type Todo } from '../../lib/ipc';
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

  // ---- calendar (Monday-first): holidays, to-dos due and countdowns on their days ----
  const cells = $derived(monthCells(month));
  const todayKey = $derived(ymd(today));
  const shift = (n: number) => (month = new Date(month.getFullYear(), month.getMonth() + n, 1));
  const weekdays = Array.from({ length: 7 }, (_, i) => new Intl.DateTimeFormat(undefined, { weekday: 'narrow' }).format(new Date(2024, 0, 1 + i)));
  const todos = live<Todo[]>('todos', ipc.todoList, []);
  const countdowns = $derived(profile().widgets.filter((w) => w.id === 'countdown')
    .flatMap((w) => (w.settings as Partial<CountdownSettings> | undefined)?.items ?? []));
  const byDay = $derived(itemsByDay(hol.all, todos.v, countdowns));
  let selected = $state(ymd(new Date()));
  const selDate = $derived(new Date(selected + 'T00:00'));
  const selItems = $derived(byDay.get(selected) ?? []);
  const selLabel = $derived(selDate.toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' })
    + (settings.calHijri ? ` · ${hijriLabel(hijriOf(selDate))}` : ''));
  const goToday = () => { month = new Date(today.getFullYear(), today.getMonth(), 1); selected = todayKey; };
  const toggleTodo = (t: Todo) => ipc.todoUpdate({ ...t, done: !t.done });
  let addingTodo = $state(false);
  let todoText = $state('');
  async function addTodo() {
    if (!todoText.trim()) return;
    // ponytail: to-dos have no all-day form, so a day's to-do is due at 9:00 that morning.
    await ipc.todoAdd(todoText, new Date(selDate.getFullYear(), selDate.getMonth(), selDate.getDate(), 9).getTime());
    todoText = ''; addingTodo = false;
  }
  const dueText = (t: Todo) => t.dueAt ? timeText(new Date(t.dueAt), settings, false) : '';
  // Today's sunrise and sunset at the shared location.
  const sun = $derived.by(() => {
    const p = here.s?.place;
    if (!p) return null;
    const t = sunTimes(p.lat, p.lon, today.getFullYear(), today.getMonth() + 1, today.getDate());
    return t.rise && t.set ? { rise: timeText(new Date(t.rise), settings, false), set: timeText(new Date(t.set), settings, false) } : null;
  });
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
    <button class="title" onclick={goToday} title="Back to today">
      {new Intl.DateTimeFormat(undefined, { month: 'long', year: 'numeric' }).format(month)}
    </button>
    <button class="icon-btn" onclick={() => shift(1)} aria-label="Next month"><ChevronRight size={16} /></button>
  </div>
  {#if sun}
    <div class="sun dim small"><Sunrise size={12} /> {sun.rise} <Sunset size={12} /> {sun.set}</div>
  {/if}
  <div class="grid" class:weeks={settings.calWeeks}>
    {#if settings.calWeeks}<span class="wd"></span>{/if}
    {#each weekdays as w, i (i)}<span class="wd dim">{w}</span>{/each}
    {#each cells as c, i (c.key)}
      {@const items = byDay.get(c.key) ?? []}
      {@const h = items.find((x) => x.kind === 'holiday')}
      {@const hj = settings.calHijri ? hijriDay(c.date) : null}
      {#if settings.calWeeks && i % 7 === 0}<span class="wk dim">{isoWeek(c.date)}</span>{/if}
      <button class="day" class:today={c.key === todayKey} class:sel={c.key === selected} class:out={!c.inMonth}
              class:holiday={!!h} class:wkend={isWeekend(c.date, hol.state)} title={h?.label}
              onclick={() => (selected = c.key)} aria-label={c.date.toDateString()} aria-pressed={c.key === selected}>
        <span class="n">{c.day}</span>
        {#if hj}<span class="hj">{hj.d === 1 ? '☾' : hj.d}</span>{/if}
        <span class="dots">
          {#if items.some((x) => x.kind === 'todo' && !x.todo?.done)}<i class="dtodo"></i>{/if}
          {#if items.some((x) => x.kind === 'countdown')}<i class="dcd"></i>{/if}
        </span>
      </button>
    {/each}
  </div>
  <div class="selh">
    <b class="ellipsis">{selLabel}</b>
    <button class="icon-btn add" class:on={addingTodo} onclick={() => (addingTodo = !addingTodo)} aria-label="Add a to-do on this day" title="Add a to-do on this day"><Plus size={14} /></button>
  </div>
  {#if addingTodo}
    <!-- svelte-ignore a11y_autofocus -->
    <input class="search" placeholder="To-do for this day (reminder at 9:00)…" bind:value={todoText} autofocus
           onkeydown={(e) => { if (e.key === 'Enter') addTodo(); if (e.key === 'Escape') addingTodo = false; }} />
  {/if}
  {#each selItems as it, i (it.kind + i)}
    <div class="item {it.kind}" class:done={it.todo?.done}>
      {#if it.kind === 'todo' && it.todo}
        <input type="checkbox" checked={it.todo.done} onchange={() => toggleTodo(it.todo!)} aria-label="Done" />
        <span class="ellipsis grow">{it.label}</span>
        <span class="dim small">{dueText(it.todo)}</span>
      {:else if it.kind === 'countdown'}
        <span class="em">{it.emoji}</span><span class="ellipsis grow">{it.label}</span><span class="dim small">countdown</span>
      {:else}
        <i class="hdot"></i><span class="ellipsis grow">{it.label}</span><span class="dim small">holiday</span>
      {/if}
    </div>
  {:else}
    {#if !addingTodo}<p class="dim small empty">Nothing on this day.</p>{/if}
  {/each}
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
  .sun { display: flex; align-items: center; gap: 4px; justify-content: center; margin: -2px 0 6px; }
  .grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 2px; text-align: center; }
  .grid.weeks { grid-template-columns: 22px repeat(7, 1fr); }
  .wd { font-size: 11px; padding: 3px 0; }
  .wk { font-size: 9px; align-self: center; }
  .day { display: flex; flex-direction: column; align-items: center; gap: 1px; padding: 3px 0 2px; border-radius: calc(8px * var(--rs));
         font-variant-numeric: tabular-nums; font-size: 12px; line-height: 1.1; }
  .day:hover { background: var(--pill-hover); }
  .hj { font-size: 8px; color: var(--fg-dim); }
  .out { opacity: 0.35; }
  .wkend:not(.today):not(.sel) { background: var(--bg-2); }
  .sel:not(.today) { box-shadow: inset 0 0 0 1.5px var(--accent); }
  .today { background: var(--accent); color: var(--on-accent); font-weight: 600; }
  .today .hj { color: var(--on-accent); opacity: 0.8; }
  .holiday:not(.today) .n { color: #ff9f43; font-weight: 600; }
  .dots { display: flex; gap: 2px; height: 4px; }
  .dots i { width: 4px; height: 4px; border-radius: 50%; }
  .dtodo { background: var(--accent); }
  .today .dtodo { background: var(--on-accent); }
  .dcd { background: #a78bfa; }
  .selh { display: flex; align-items: center; justify-content: space-between; gap: 6px; margin-top: 8px; font-size: 12px; padding: 0 2px; }
  .item { display: flex; align-items: center; gap: 8px; padding: 5px 6px; border-radius: var(--r-inner); font-size: 12px; }
  .item:hover { background: var(--pill-hover); }
  .item.done .grow { text-decoration: line-through; opacity: 0.6; }
  .item input { margin: 0; }
  .em { width: 16px; text-align: center; }
  .hdot { width: 8px; height: 8px; border-radius: 50%; background: #ff9f43; flex: none; margin: 0 4px; }
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
