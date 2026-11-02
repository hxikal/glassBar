<script lang="ts">
  // Settings › Performance: what GlassBar itself uses (its whole process tree), an hour of
  // history, and the Low / Normal / High modes with what each one changes, widget by widget.
  import { onMount } from 'svelte';
  import { Check } from '@lucide/svelte';
  import Sparkline from '../lib/Sparkline.svelte';
  import Boost from '../lib/Boost.svelte';
  import { update } from '../lib/config.svelte';
  import { events, ipc, type PerfMode, type PerfSample, type ProcUsage } from '../lib/ipc';
  import { perf } from '../lib/perf.svelte';
  import { widgets } from '../widgets/registry';
  import { APP_ROWS, MODE_CARDS, ago, changeList, fmtBytes, fmtMetric, fmtPct, netLastHour, pointAt, timeSeries, type Metric } from './perfPage';

  let samples = $state<PerfSample[]>([]);
  // The newest per-process reading; samples drop theirs (an hour of them is dead weight).
  let procs = $state<ProcUsage[]>([]);
  let metric = $state<Metric>('ram');
  let graphW = $state(520);
  let hover = $state<number | null>(null);
  const STEP = 30_000, H = 70;

  onMount(() => {
    ipc.perfHistory().then((h) => { samples = h; procs = h.at(-1)?.procs ?? []; }).catch(() => {});
    const off = events.perfSample((s) => {
      procs = s.procs;
      samples = [...samples.filter((x) => x.at > s.at - 3_600_000), { ...s, procs: [] }];
    });
    // The 1-second sampling is a lease: renew it while this page is open.
    ipc.perfWatch().catch(() => {});
    const t = setInterval(() => ipc.perfWatch().catch(() => {}), 2000);
    return () => { clearInterval(t); off.then((f) => f()); };
  });

  const last = $derived(samples.at(-1));
  // One point per 30 s, so the hour is spaced by time whatever the sampling rate was.
  const series = $derived(timeSeries(samples, metric, STEP));
  const floor: Record<Metric, number> = { ram: 50, cpu: 5, gpu: 5, net: 1024 };
  const max = $derived(Math.max(floor[metric], ...series) * 1.2);
  const changes = $derived(changeList(perf.mode, [...APP_ROWS, ...Object.values(widgets).map((m) => ({ name: m.name, perf: m.perf }))]));
  // The point under the pointer: where to draw the marker, and what it says.
  const tip = $derived.by(() => {
    if (hover === null || hover >= series.length) return null;
    const v = series[hover], n = series.length;
    const t = samples[0].at + hover * STEP;
    const clock = new Date(t).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
    return { x: n > 1 ? (hover * graphW) / (n - 1) : graphW, y: H - (Math.min(max, Math.max(0, v)) / max) * H,
             text: fmtMetric(metric, v), when: `${clock} · ${ago(Date.now() - t)}` };
  });
  const onMove = (e: PointerEvent) => {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    hover = series.length ? pointAt(e.clientX - r.left, r.width, series.length) : null;
  };
  const mb = (v: number) => `${v.toFixed(1)} MB`;
  const title = (m: PerfMode) => MODE_CARDS.find((c) => c.mode === m)!.title;

  const pick = (mode: PerfMode) => update((c) => { c.performance = { mode, auto: false }; });
  const setAuto = (auto: boolean) => update((c) => { c.performance.auto = auto; });
</script>

<div class="group">
  <div class="strip">
    <span>RAM <b>{last ? mb(last.ramMb) : '—'}</b></span>
    <span>CPU <b>{last ? fmtPct(last.cpu) : '—'}</b></span>
    <span>GPU <b>{last ? fmtPct(last.gpu) : '—'}</b></span>
    <span>Net <b>{fmtBytes(netLastHour(samples))}/hr</b> <span class="dim">({fmtBytes(last?.netBytes ?? 0)} since start)</span></span>
  </div>
  <div class="seg chips">
    {#each [['ram', 'RAM'], ['cpu', 'CPU'], ['gpu', 'GPU'], ['net', 'Network']] as const as [k, label] (k)}
      <button class:sel={metric === k} onclick={() => (metric = k)}>{label}</button>
    {/each}
    {#if tip}<span class="span read"><b>{tip.text}</b> <span class="dim">{tip.when}</span></span>
    {:else}<span class="dim span">Last hour</span>{/if}
  </div>
  <div class="graph" bind:clientWidth={graphW} onpointermove={onMove} onpointerleave={() => (hover = null)} role="img"
       aria-label="{metric.toUpperCase()} over the last hour">
    <Sparkline values={series} {max} width={graphW} height={H} />
    {#if tip}
      <i class="cursor" style="left:{tip.x}px"></i>
      <i class="dot" style="left:{tip.x}px; top:{tip.y}px"></i>
    {/if}
  </div>
  {#if procs.length}
    <table class="procs">
      <thead><tr><th>Process</th><th>RAM</th><th>CPU</th><th>GPU</th></tr></thead>
      <tbody>
        {#each procs as p (p.pid)}
          <tr><td>{p.name}</td><td>{mb(p.ramMb)}</td><td>{fmtPct(p.cpu)}</td><td>{fmtPct(p.gpu)}</td></tr>
        {/each}
      </tbody>
    </table>
  {/if}
  <p class="hint">GlassBar and its WebView2 helpers (this Settings window too), measured like Task Manager's Memory column.
    Apps you open from the dock aren't counted. Network counts data GlassBar downloaded.</p>
</div>

<h2>Boost</h2>
<Boost />

<h2>Mode</h2>
<div class="group">
  <label class="field"><span>Auto</span>
    <input type="checkbox" checked={perf.auto} onchange={(e) => setAuto(e.currentTarget.checked)} />
  </label>
  <p class="hint">Drops to Low while your PC is short on memory or CPU, or battery saver is on, and back to Normal once it's calm.</p>
  <div class="cards">
    {#each MODE_CARDS as c (c.mode)}
      <button class="card" class:sel={perf.mode === c.mode} aria-pressed={perf.mode === c.mode} onclick={() => pick(c.mode)}>
        <b>{c.title}{#if perf.mode === c.mode}<em><Check size={12} strokeWidth={3} /> Active</em>{/if}</b><span>{c.summary}</span>
      </button>
    {/each}
  </div>
  {#if perf.auto && perf.reason}<p class="reason">Auto chose {title(perf.mode)} · {perf.reason}</p>{/if}
</div>

<h2>{title(perf.mode)} {perf.mode === 'normal' ? 'compared to High' : 'compared to Normal'}</h2>
<div class="group">
  {#each changes.rows as r (r.name)}
    <div class="field"><span>{r.name}</span><span class="dim">{r.text}</span></div>
  {/each}
  <p class="hint">Unchanged: {changes.unchanged.join(', ')}</p>
</div>

<style>
  .strip { display: flex; flex-wrap: wrap; gap: 6px 18px; padding: 8px 0 2px; font-size: 13px; }
  .strip b { font-variant-numeric: tabular-nums; }
  .dim { color: var(--fg-dim); }
  .chips { align-self: flex-start; align-items: center; }
  .chips button { text-transform: none; }
  .chips .span { padding: 0 8px; font-size: 12px; }
  .graph { position: relative; width: 100%; height: 70px; cursor: crosshair; }
  .graph i { position: absolute; pointer-events: none; }
  .cursor { top: 0; bottom: 0; width: 1px; background: var(--fg-dim); opacity: 0.5; }
  .dot { width: 8px; height: 8px; margin: -4px 0 0 -4px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 0 2px var(--fg); }
  .read b { font-variant-numeric: tabular-nums; }
  .procs { width: 100%; margin: 10px 0 12px; border-collapse: collapse; font-size: 12px; }
  .procs th { font-weight: 500; color: var(--fg-dim); text-align: right; padding: 0 0 4px 12px; }
  .procs td { text-align: right; padding: 4px 0 4px 12px; border-top: 1px solid var(--line-soft); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .procs :is(th, td):first-child { text-align: left; padding-left: 0; width: 100%; white-space: normal; }
  .cards { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; padding: 4px 0 8px; }
  .card { display: flex; flex-direction: column; gap: 3px; text-align: left; padding: 10px 12px; border-radius: calc(10px * var(--rs));
          background: var(--pill); border: 2px solid transparent; }
  .card:hover { background: var(--pill-hover); }
  .card.sel { background: var(--accent); color: var(--on-accent); }
  .card b { display: flex; align-items: center; justify-content: space-between; gap: 6px; }
  .card em { display: inline-flex; align-items: center; gap: 3px; font-style: normal; font-size: 11px; font-weight: 600;
             padding: 2px 7px; border-radius: 999px; background: color-mix(in srgb, var(--on-accent) 22%, transparent); }
  .card span { font-size: 12px; color: var(--fg-dim); }
  .card.sel span { color: var(--on-accent); opacity: 0.85; }
  .reason { margin: 0 0 8px; padding: 6px 10px; border-radius: calc(8px * var(--rs)); font-size: 12px;
            background: color-mix(in srgb, #ffb340 18%, transparent); }
</style>
