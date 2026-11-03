<script lang="ts">
  // Tasbih in the prayer panel: a big tap ring (Space / Enter count too), the dhikr, the
  // target, undo, and reset by press-and-hold so a stray click never loses a count. The header
  // pins it beside the prayer pill on the dock, or hides it (the panel's options bring it back).
  import { EyeOff, Pin, RotateCcw, Undo2 } from '@lucide/svelte';
  import { updateWidgetSettings } from '../../lib/config.svelte';
  import { ipc, type TasbihState } from '../../lib/ipc';
  import { live } from '../../lib/live.svelte';
  import { tick } from '../../lib/sound';
  import type { SolatSettings } from './index';
  import { DHIKR, TARGETS, phraseOf, stepsOf } from './tasbih';

  let { instance, settings }: { instance: string; settings: SolatSettings } = $props();
  const t = live<TasbihState>('tasbih', ipc.tasbihGet, { count: 0, round: 0, step: 0, today: 0, day: '' });
  const dhikr = $derived(settings.dhikr ?? 'subhanallah');
  const steps = $derived(stepsOf(dhikr, settings.tasbihTarget ?? 33));
  const target = $derived(steps[Math.min(t.v.step, steps.length - 1)] ?? 33);
  const done = $derived(t.v.count >= target);
  const R = 52, C = 2 * Math.PI * R;
  let pop = $state(false);
  let custom = $state(false);

  async function add() {
    const before = done;
    const s = await ipc.tasbihAdd(1, steps).catch(() => null);
    if (!s) return;
    t.v = s;
    pop = false; requestAnimationFrame(() => (pop = true));
    if (!before && s.count >= steps[Math.min(s.step, steps.length - 1)]) tick();
  }
  async function undo() { const s = await ipc.tasbihAdd(-1, steps).catch(() => null); if (s) t.v = s; }

  // Reset: hold 600 ms (a ring fills while held).
  let holding = $state(false);
  let holdTimer: ReturnType<typeof setTimeout>;
  function holdStart() { holding = true; holdTimer = setTimeout(async () => { holding = false; t.v = await ipc.tasbihReset(); }, 600); }
  function holdEnd() { holding = false; clearTimeout(holdTimer); }
  const setTarget = (n: number) => { custom = false; if (n > 0) updateWidgetSettings(instance, { tasbihTarget: Math.min(100_000, Math.round(n)) }); };
</script>

<section class="tas">
  <div class="head">
    <h4>Tasbih</h4>
    <span class="dim today">Today {t.v.today.toLocaleString()}</span>
    <button class="mini" class:on={settings.tasbihOnDock !== false} aria-pressed={settings.tasbihOnDock !== false}
            onclick={() => updateWidgetSettings(instance, { tasbihOnDock: settings.tasbihOnDock === false })}
            title={settings.tasbihOnDock !== false ? 'On the dock beside the prayer pill: click to remove' : 'Show on the dock beside the prayer pill'}
            aria-label="Tasbih on the dock"><Pin size={13} /></button>
    <button class="mini" onclick={() => updateWidgetSettings(instance, { showTasbih: false })}
            title="Hide Tasbih from this panel (the options button above brings it back)" aria-label="Hide Tasbih from this panel"><EyeOff size={13} /></button>
  </div>

  <button class="tap" class:done class:pop onclick={add} onanimationend={() => (pop = false)} aria-label="Count. {t.v.count} of {target}">
    <svg viewBox="0 0 120 120" width="120" height="120" aria-hidden="true">
      <circle cx="60" cy="60" r={R} class="track" />
      <circle cx="60" cy="60" r={R} class="arc" stroke-dasharray={C} stroke-dashoffset={C * (1 - Math.min(1, t.v.count / target))} />
    </svg>
    <span class="num"><b>{t.v.count}</b><span class="dim">of {target}</span></span>
  </button>
  <p class="phrase">{phraseOf(dhikr, t.v.step)}{#if t.v.round}<span class="dim"> · round {t.v.round + 1}</span>{/if}</p>

  <div class="ctrls">
    <select value={dhikr} onchange={(e) => updateWidgetSettings(instance, { dhikr: e.currentTarget.value as SolatSettings['dhikr'] })} aria-label="Dhikr">
      {#each DHIKR as d (d.key)}<option value={d.key}>{d.label}</option>{/each}
    </select>
    <button class="ib" onclick={undo} disabled={t.v.count === 0} aria-label="Undo one" title="Undo (−1)"><Undo2 size={15} /></button>
    <button class="ib hold" class:holding onpointerdown={holdStart} onpointerup={holdEnd} onpointerleave={holdEnd}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); if (!holding) holdStart(); } }} onkeyup={holdEnd}
            aria-label="Reset (hold)" title="Hold to reset"><RotateCcw size={15} /></button>
  </div>
  {#if dhikr !== 'tasbih_fatimah'}
    <div class="targets" role="radiogroup" aria-label="Target">
      {#each TARGETS as n (n)}
        <button role="radio" aria-checked={(settings.tasbihTarget ?? 33) === n} class:sel={(settings.tasbihTarget ?? 33) === n} onclick={() => setTarget(n)}>{n.toLocaleString()}</button>
      {/each}
      {#if custom}
        <!-- svelte-ignore a11y_autofocus -->
        <input type="number" min="1" max="100000" autofocus placeholder="Target" aria-label="Own target"
               onkeydown={(e) => { if (e.key === 'Enter') setTarget(+e.currentTarget.value); if (e.key === 'Escape') { e.stopPropagation(); custom = false; } }}
               onblur={(e) => setTarget(+e.currentTarget.value)} />
      {:else}
        <button class:sel={!TARGETS.includes(settings.tasbihTarget ?? 33)} onclick={() => (custom = true)} title="Your own target">
          {TARGETS.includes(settings.tasbihTarget ?? 33) ? '⋯' : (settings.tasbihTarget ?? 33).toLocaleString()}
        </button>
      {/if}
    </div>
  {:else}
    <p class="dim small">33 Subhanallah · 33 Alhamdulillah · 34 Allahuakbar</p>
  {/if}
</section>

<svelte:window onkeydown={(e) => { if ((e.key === ' ' || e.key === 'Enter') && (e.target as HTMLElement)?.tagName === 'BODY') { e.preventDefault(); add(); } }} />

<style>
  .tas { margin-top: var(--s-4); padding-top: var(--s-3); border-top: 1px solid var(--border); display: flex; flex-direction: column; align-items: center; }
  .head { display: flex; align-items: center; gap: 2px; width: 100%; }
  .head .today { margin: 0 4px 0 auto; }
  .mini { display: grid; place-items: center; width: 24px; height: 24px; border-radius: calc(7px * var(--rs)); color: var(--fg-dim); transition: background 0.2s var(--ease), color 0.2s var(--ease); }
  .mini:hover { background: var(--pill-hover); color: var(--fg); }
  .mini.on { color: var(--accent); }
  .mini:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
  h4 { margin: 0 2px; font-size: 13px; font-weight: 600; }
  .today { font-size: 12px; font-variant-numeric: tabular-nums; }
  .tap { position: relative; display: grid; place-items: center; width: 120px; height: 120px; margin: 10px 0 4px; border-radius: 50%;
         background: var(--bg-2); transition: background 0.2s var(--ease), transform 0.12s var(--ease); }
  .tap:hover { background: var(--pill-hover); }
  .tap:active { transform: scale(0.96); }
  .tap:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
  .tap svg { position: absolute; inset: 0; transform: rotate(-90deg); }
  .track { fill: none; stroke: var(--track); stroke-width: 6; }
  .arc { fill: none; stroke: var(--accent); stroke-width: 6; stroke-linecap: round; transition: stroke-dashoffset 0.25s var(--ease); }
  .num { position: relative; display: flex; flex-direction: column; align-items: center; line-height: 1.1; }
  .num b { font-size: 32px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .num .dim { font-size: 12px; font-variant-numeric: tabular-nums; }
  .done .arc { stroke: #30d158; }
  .done { background: color-mix(in srgb, #30d158 14%, var(--bg-2)); }
  .pop { animation: bump 0.14s var(--ease); }
  @keyframes bump { 50% { transform: scale(0.95); } }
  .phrase { margin: 4px 0 10px; font-size: 14px; font-weight: 500; }
  .ctrls { display: flex; align-items: center; gap: 6px; width: 100%; }
  .ctrls select { flex: 1; height: 36px; border-radius: var(--r-inner); background: var(--bg-2); border: 1px solid var(--border); color: var(--fg); padding: 0 8px; font: inherit; font-size: 13px; }
  .ib { position: relative; display: grid; place-items: center; width: 36px; height: 36px; border-radius: var(--r-inner); background: var(--bg-2); overflow: hidden; }
  .ib:hover:not(:disabled) { background: var(--pill-hover); }
  .ib:disabled { opacity: 0.4; }
  .ib:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
  /* Hold-to-reset: a red fill rises while held. */
  .hold::before { content: ''; position: absolute; inset: auto 0 0 0; height: 0; background: color-mix(in srgb, #ff453a 35%, transparent); }
  .hold.holding::before { height: 100%; transition: height 0.6s linear; }
  .targets { display: flex; gap: 2px; width: 100%; margin-top: 8px; padding: 3px; border-radius: var(--r-inner); background: var(--pill); }
  .targets button, .targets input { flex: 1; min-width: 0; height: 28px; border-radius: calc(8px * var(--rs)); font-size: 12px; font-variant-numeric: tabular-nums; }
  .targets .sel { background: var(--accent); color: var(--on-accent); font-weight: 600; }
  .targets input { background: var(--bg-2); border: none; color: var(--fg); text-align: center; font: inherit; font-size: 12px; }
  .small { font-size: 11.5px; margin: 8px 0 0; }
  @media (prefers-reduced-motion: reduce) { .pop { animation: none; } .hold.holding::before { transition: none; } }
</style>
