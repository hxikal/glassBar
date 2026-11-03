<script lang="ts">
  // A drop that fills with today's water, and the amount (or %) beside it. Click for the panel.
  import type { PillProps } from '../types';
  import type { WaterSettings } from './settings';
  import { fmtWater, progressOf, untilText } from './units';
  import { liveWater } from './live.svelte';
  import Drop from './Drop.svelte';
  import Tank from '../../lib/Tank.svelte';

  let { size, settings, vertical }: PillProps<WaterSettings> = $props();
  const w = liveWater(() => settings);
  const p = $derived(w.v ? progressOf(w.v.total, w.v.goal) : 0);
  const text = $derived(!w.v ? '' : settings.show === 'percent' ? `${Math.round(p * 100)}%` : fmtWater(w.v.total, settings.unit));
  // Time to the next reminder, so you can see the reminders are on and working.
  const next = $derived(w.v?.nextReminder != null ? untilText(w.v.nextReminder - w.now) : null);
  const title = $derived(w.v ? `Water today: ${fmtWater(w.v.total, settings.unit)} of ${fmtWater(w.v.goal, settings.unit)}`
    + (next ? ` · next reminder ${next === 'now' ? 'now' : `in ${next}`}` : settings.remind === false ? ' · reminders off' : '') : 'Water');

  // A drink was logged (here, in the panel, or from the dock alert): "+250 ml" floats up.
  let added = $state<{ n: number; ml: number } | null>(null);
  let prev: number | null = null;
  let addTimer: ReturnType<typeof setTimeout>;
  $effect(() => {
    const t = w.v?.total;
    if (t === undefined) return;
    if (prev !== null && t > prev) {
      added = { n: (added?.n ?? 0) + 1, ml: t - prev };
      clearTimeout(addTimer);
      addTimer = setTimeout(() => (added = null), 1300);
    }
    prev = t;
  });
  $effect(() => () => clearTimeout(addTimer));
</script>

{#if vertical}
  <div class="side wside" {title}>
    <Tank fill={p} />
    <Drop fill={p} size={20} />
    <span class="side-v">{Math.round(p * 100)}<small>%</small></span>
  </div>
{:else}
  <!-- The whole pill fills with water up to today's level. -->
  <div class="pill water" class:done={p >= 1} {title}>
    <Tank fill={p} />
    <Drop fill={p} size={18} />
    {#if size !== 'small' && text}<span class="v">{text}</span>{/if}
    {#if size !== 'small' && next}<span class="next">{next}</span>{/if}
    {#if added}
      {#key added.n}<span class="plus" aria-hidden="true">+{fmtWater(added.ml, settings.unit)}</span>{/key}
    {/if}
  </div>
{/if}

<style>
  .water { gap: 6px; padding: 0 10px 0 8px; }
  .v { font-size: 12px; font-weight: 600; font-variant-numeric: tabular-nums; white-space: nowrap; }
  .done .v { color: #40c8e0; }
  .next { font-size: 10.5px; font-variant-numeric: tabular-nums; color: var(--fg-dim); white-space: nowrap; }
  .water, .wside { position: relative; overflow: visible; }
  /* Content above the water. */
  .water > :global(:not(.tank)), .wside > :global(:not(.tank)) { position: relative; z-index: 1; }
  /* "+250 ml" rising out of the pill and fading. */
  .plus {
    position: absolute; left: 50%; top: 0; translate: -50% 0; pointer-events: none; white-space: nowrap;
    padding: 1px 6px; border-radius: var(--r-chip); background: #40c8e0; color: #0b2a33; font-size: 10px; font-weight: 600;
    animation: float 1.3s cubic-bezier(0.22, 1, 0.36, 1) forwards;
  }
  @keyframes float { 0% { opacity: 0; transform: translateY(6px) scale(0.8); } 20% { opacity: 1; transform: translateY(-2px) scale(1); } 100% { opacity: 0; transform: translateY(-14px); } }
  .v { transition: color 0.3s var(--ease); }
  @media (prefers-reduced-motion: reduce) { .plus { animation-duration: 0.01s; } }
</style>
