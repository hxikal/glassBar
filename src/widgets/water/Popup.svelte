<script lang="ts">
  // Water: today's progress ring, one-tap cups (with Undo), today's drinks, the last 7 days,
  // and the goal / reminder settings behind the gear.
  import { Plus, Settings2, Undo2, X } from '@lucide/svelte';
  import { ipc } from '../../lib/ipc';
  import { updateWidgetSettings } from '../../lib/config.svelte';
  import OptionsPopover from '../../lib/OptionsPopover.svelte';
  import Tank from '../../lib/Tank.svelte';
  import type { PopupProps } from '../types';
  import { WATER_DEFAULTS, type WaterSettings } from './settings';
  import { fmtWater, progressOf, untilText } from './units';
  import { liveWater } from './live.svelte';

  let { instance, settings: raw }: PopupProps<WaterSettings> = $props();
  const s = $derived({ ...WATER_DEFAULTS, ...raw });
  const w = liveWater(() => raw);
  let showOptions = $state(false);
  const p = $derived(w.v ? progressOf(w.v.total, w.v.goal) : 0);
  const cup = $derived(Number(s.cup) || 250);
  const cups = $derived([...new Set([150, 250, 500, cup])].sort((a, b) => a - b));

  // Undo: the drink just logged, for 8 s.
  let undo = $state<number | null>(null);
  let undoTimer: ReturnType<typeof setTimeout>;
  async function add(ml: number) {
    const t = await ipc.waterAdd(ml).catch(() => null);
    const last = t?.entries.at(-1);
    if (!last) return;
    undo = last.at;
    clearTimeout(undoTimer);
    undoTimer = setTimeout(() => (undo = null), 8000);
  }
  function takeBack() {
    if (undo === null) return;
    ipc.waterRemove(undo).catch(() => {});
    undo = null;
  }
  $effect(() => () => clearTimeout(undoTimer));

  const time = (at: number) => new Date(at).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
  const dayName = (d0: number) => new Date(d0).toLocaleDateString([], { weekday: 'narrow' });
  const weekMax = $derived(Math.max(w.v?.goal ?? 1, ...(w.v?.week ?? []).map((d) => d.ml)));
  const save = (patch: Record<string, unknown>) => updateWidgetSettings(instance, patch);
</script>

<!-- The whole panel is the tank: it fills up from empty to today's level as the panel opens,
     keeps moving gently while open, and rises with a roll when a drink is logged. -->
{#if w.v}<Tank fill={p} fromEmpty live fixed />{/if}
<div class="wbody">

<button class="icon-btn gear" class:on={showOptions} onclick={() => (showOptions = !showOptions)} aria-label="Water options"><Settings2 size={16} /></button>

{#if showOptions}
  <div class="opts">
    <OptionsPopover title="Goal" values={s} save={save} options={[
      { type: 'choice', key: 'goalMode', label: 'Daily goal', choices: [
        { value: 'fixed', label: 'Fixed amount' },
        { value: 'dynamic', label: 'From my weight', hint: '35 ml per kg; +10 % on a hot day when the Weather widget knows the temperature' }] },
      ...(s.goalMode === 'fixed'
        ? [{ type: 'range' as const, key: 'goal', label: 'Goal', min: 1000, max: 5000, step: 100, unit: ' ml' }]
        : [{ type: 'range' as const, key: 'weight', label: 'Weight', min: 30, max: 150, step: 1, unit: ' kg' },
           { type: 'toggle' as const, key: 'hotBonus', label: 'More on hot days', hint: '+10 % at 30 °C or more' }]),
      { type: 'choice', key: 'cup', label: 'Usual cup', choices: [150, 200, 250, 330, 500].map((v) => ({ value: String(v), label: `${v} ml` })) },
      { type: 'choice', key: 'unit', label: 'Show in', choices: [{ value: 'ml', label: 'ml' }, { value: 'oz', label: 'fl oz' }] },
      { type: 'choice', key: 'show', label: 'On the dock', choices: [{ value: 'amount', label: 'Amount' }, { value: 'percent', label: 'Percent' }] },
    ]} />
    <OptionsPopover title="Reminders" values={s} save={save} options={[
      { type: 'toggle', key: 'remind', label: 'Remind me to drink', hint: 'The dock shows it, with buttons to log a cup or wait 15 minutes' },
      { type: 'range', key: 'every', label: 'Every', min: 15, max: 180, step: 15, unit: ' min after a drink' },
      { type: 'range', key: 'fromHour', label: 'From', min: 0, max: 23, step: 1, unit: ':00' },
      { type: 'range', key: 'toHour', label: 'Until', min: 1, max: 24, step: 1, unit: ':00' },
    ]} />
  </div>
{/if}

{#if w.v}
  <div class="hero">
    <span class="in">
      <b>{fmtWater(w.v.total, s.unit)}</b>
      <span class="dim">of {fmtWater(w.v.goal, s.unit)} · {Math.round(p * 100)}%</span>
    </span>
    {#if p >= 1}<p class="done">Goal reached today</p>{:else if w.v.hot}<p class="dim small">Hot day: goal +10 %</p>{/if}
    {#if !s.remind}
      <p class="dim small">Reminders are off <button class="turn-on" onclick={() => save({ remind: true })}>Turn on</button></p>
    {:else if w.v.nextReminder != null}
      {@const left = untilText(w.v.nextReminder - w.now)}
      <p class="dim small">Next reminder {left === 'now' ? 'now' : `in ${left}`} · {new Date(w.v.nextReminder).toDateString() === new Date(w.now).toDateString() ? '' : 'tomorrow '}{time(w.v.nextReminder)}</p>
    {/if}
  </div>

  <div class="cups">
    {#each cups as ml (ml)}
      <button class="cup" class:main={ml === cup} onclick={() => add(ml)}><Plus size={12} />{fmtWater(ml, s.unit)}</button>
    {/each}
  </div>
  {#if undo !== null}
    <button class="undo" onclick={takeBack}><Undo2 size={12} /> Undo</button>
  {/if}

  {#if w.v.entries.length}
    <h4 class="dim">Today</h4>
    <div class="log">
      {#each [...w.v.entries].reverse() as e (e.at)}
        <div class="row">
          <span class="t dim">{time(e.at)}</span>
          <span class="ml">{fmtWater(e.ml, s.unit)}</span>
          <button class="rm" onclick={() => ipc.waterRemove(e.at)} aria-label="Remove this drink"><X size={12} /></button>
        </div>
      {/each}
    </div>
  {/if}

  <h4 class="dim">Last 7 days</h4>
  <div class="week" role="img" aria-label="Water per day, last 7 days">
    {#each w.v.week as d, i (d.day0)}
      <span class="day" title="{new Date(d.day0).toLocaleDateString()}: {fmtWater(d.ml, s.unit)}">
        <span class="bar" class:today={i === w.v.week.length - 1} class:met={d.ml >= w.v.goal} style:height="{Math.max(3, (d.ml / weekMax) * 100)}%"></span>
        <span class="dn dim">{dayName(d.day0)}</span>
      </span>
    {/each}
    <span class="goalline" style:bottom="calc({(w.v.goal / weekMax) * 100}% * 0.78 + 16px)" aria-hidden="true"></span>
  </div>
  <p class="foot dim">The day starts again at midnight.</p>
{:else}
  <p class="dim small skeleton">Loading…</p>
{/if}
</div>

<style>
  .gear { position: absolute; top: 10px; right: 10px; z-index: 2; }
  .gear.on { color: var(--accent); }
  .opts { margin: 28px 0 8px; display: flex; flex-direction: column; gap: 8px; }
  .wbody { position: relative; z-index: 1; }
  .hero { display: flex; flex-direction: column; align-items: center; gap: 6px; margin: 40px 0 18px; }
  .in { display: flex; flex-direction: column; align-items: center; line-height: 1.2; }
  .in b { font-size: 34px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .in .dim { font-size: 12px; }
  .done { margin: 0; color: #40c8e0; font-size: 12px; font-weight: 600; animation: rise 0.3s var(--ease); }
  @keyframes rise { from { opacity: 0; transform: translateY(4px); } }
  .small { font-size: 11px; margin: 0; }
  .turn-on { margin-left: 4px; padding: 1px 8px; border-radius: var(--r-chip); background: var(--accent); color: var(--on-accent); font-size: 11px; font-weight: 600; }
  .cups { display: flex; gap: 6px; justify-content: center; }
  .cup {
    display: inline-flex; align-items: center; gap: 4px; height: 32px; padding: 0 12px; border-radius: var(--r-chip);
    background: var(--pill); border: 1px solid var(--line-soft); font-size: 12px; font-weight: 500; font-variant-numeric: tabular-nums;
    transition: background 0.2s var(--ease), transform 0.15s var(--ease);
  }
  .cup:hover { background: var(--pill-hover); }
  .cup:active { transform: scale(0.95); }
  .cup.main { background: #40c8e0; border-color: transparent; color: #0b2a33; }
  .cup:focus-visible, .undo:focus-visible, .rm:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .undo { display: flex; align-items: center; gap: 4px; margin: 8px auto 0; font-size: 11.5px; color: var(--accent); animation: rise 0.2s var(--ease); }
  h4 { margin: 14px 2px 6px; font-size: 12px; font-weight: 500; }
  .log { display: flex; flex-direction: column; gap: 2px; max-height: 140px; overflow-y: auto; }
  .row { display: flex; align-items: center; gap: 10px; height: 32px; padding: 0 6px 0 10px; border-radius: var(--r-inner); background: var(--bg-2, var(--pill)); }
  .t { width: 64px; font-size: 11.5px; font-variant-numeric: tabular-nums; }
  .ml { flex: 1; font-size: 12.5px; font-weight: 500; font-variant-numeric: tabular-nums; }
  .rm { display: grid; place-items: center; width: 24px; height: 24px; border-radius: 50%; color: var(--fg-dim); opacity: 0.6; }
  .rm:hover { background: var(--pill-hover); opacity: 1; }
  .week { position: relative; display: grid; grid-template-columns: repeat(7, 1fr); gap: 6px; height: 90px; align-items: end; }
  .day { display: flex; flex-direction: column; align-items: center; justify-content: flex-end; gap: 4px; height: 100%; }
  .bar { width: 100%; max-width: 22px; max-height: 78%; border-radius: calc(5px * var(--rs)); background: var(--track); transition: height 0.5s var(--ease); }
  .bar.met { background: color-mix(in srgb, #40c8e0 55%, var(--track)); }
  .bar.today { background: #40c8e0; }
  .dn { font-size: 10px; height: 12px; }
  .goalline { position: absolute; left: 0; right: 0; border-top: 1px dashed color-mix(in srgb, #40c8e0 60%, transparent); pointer-events: none; }
  .foot { margin: 10px 0 0; font-size: 10.5px; text-align: center; }
  @media (prefers-reduced-motion: reduce) { .done, .undo { animation: none; } .bar { transition: none; } }
</style>
