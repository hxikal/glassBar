<script lang="ts">
  // Boost: trims idle apps' memory on demand (perf/boost.rs). Never closes anything; the
  // optional "close apps I'm not using" list is separate and closes only what you tick.
  // `compact`: the System Monitor popup's one-button version.
  import { Rocket, X } from '@lucide/svelte';
  import { ipc, type BoostReport, type IdleApp } from './ipc';
  import { store, update } from './config.svelte';
  import { gb, parseNames, ramLine, skippedLine } from './boost';

  let { compact = false }: { compact?: boolean } = $props();
  let lowerHogs = $state(false);
  let running = $state(false);
  let report = $state<BoostReport | null>(null);
  let error = $state('');

  async function boost() {
    running = true; error = '';
    try { report = await ipc.boostRun(!compact && lowerHogs); } catch (e) { error = String(e); } finally { running = false; }
  }

  // Never-touch list, saved as typed once the field loses focus.
  const never = $derived((store.config?.performance.neverTouch ?? []).join(', '));
  const saveNever = (s: string) => update((c) => { c.performance.neverTouch = parseNames(s); });

  // Close apps I'm not using: found on request, each ticked by hand.
  let idle = $state<IdleApp[] | null>(null);
  let picked = $state<Set<string>>(new Set());
  let finding = $state(false);
  let closing = $state(false);
  let closeMsg = $state('');
  async function find() {
    finding = true; closeMsg = ''; picked = new Set();
    try { idle = await ipc.boostIdleApps(); } catch (e) { closeMsg = String(e); } finally { finding = false; }
  }
  const tick = (exe: string) => { const n = new Set(picked); if (!n.delete(exe)) n.add(exe); picked = n; };
  const nameOf = (exe: string) => idle?.find((a) => a.exe === exe)?.name ?? exe;
  async function closePicked() {
    closing = true;
    try {
      const r = await ipc.boostClose([...picked]);
      closeMsg = [r.closed.length ? `Closed ${r.closed.map(nameOf).join(', ')}.` : '',
                  r.stillOpen.length ? `Still open: ${r.stillOpen.map(nameOf).join(', ')}. It may be asking you to save; GlassBar won't force it.` : ''].join(' ');
      idle = idle?.filter((a) => !r.closed.includes(a.exe)) ?? null;
      picked = new Set();
    } catch (e) { closeMsg = String(e); } finally { closing = false; }
  }
</script>

{#if compact}
  <div class="compact">
    <button class="btn" onclick={boost} disabled={running}><Rocket size={13} /> {running ? 'Boosting…' : 'Boost'}</button>
    <span class="dim small">{#if report}{gb(Math.max(0, report.before - report.after))} freed · {report.trimmed} idle processes trimmed{:else if error}{error}{:else}Frees memory idle apps hold. Closes nothing.{/if}</span>
  </div>
{:else}
  <div class="group">
    <p class="hint top">Windows already manages memory well. Boost only gives back memory that <b>idle</b> background apps are holding but not using: it trims their working sets, and each app gets its pages back the moment it needs them. GlassBar also drops its own caches. Nothing is closed, paused or force-quit. It runs only when you press it.</p>
    <label class="field"><span>Also lower background apps that hog the CPU<small class="dim">Set to Below normal priority; back to Normal when GlassBar quits or starts again</small></span>
      <input type="checkbox" bind:checked={lowerHogs} />
    </label>
    <div class="field"><span>Never touch<small class="dim">Exe names, comma separated, e.g. Spotify.exe, obs64</small></span>
      <input class="names" value={never} placeholder="Spotify.exe, obs64" onchange={(e) => saveNever(e.currentTarget.value)} />
    </div>
    <div class="field"><span></span>
      <button class="btn" onclick={boost} disabled={running}><Rocket size={14} /> {running ? 'Boosting…' : 'Boost'}</button>
    </div>
    {#if error}<p class="msg bad">{error}</p>{/if}
    {#if report}
      <div class="result">
        <b>{ramLine(report)}</b>
        <span>Trimmed {report.trimmed} idle processes ({gb(report.trimmedBytes)} of working sets) and GlassBar's own ({gb(report.ownBytes)}).</span>
        {#if report.top.length}
          <table>
            <tbody>{#each report.top as t (t.name)}<tr><td>{t.name}</td><td>{gb(t.bytes)}</td></tr>{/each}</tbody>
          </table>
        {/if}
        {#if report.lowered.length}<span>Lowered to Below normal until GlassBar restarts: {report.lowered.join(', ')}.</span>{/if}
        <span class="dim">{skippedLine(report)} Trimmed apps refill as they wake, so RAM in use creeps back up over time.</span>
      </div>
    {/if}
  </div>

  <h2>Close apps I'm not using</h2>
  <div class="group">
    <p class="hint top">Optional, and separate from Boost: heavy apps with windows you haven't touched in 10 minutes. Tick the ones to close; each is asked to close normally, so it can ask you to save. An app that doesn't close is left alone.</p>
    {#if idle}
      {#each idle as a (a.exe)}
        <label class="field"><span>{a.name}<small class="dim">{a.windows} window{a.windows === 1 ? '' : 's'} · {a.exe}</small></span>
          <span class="row"><span class="num">{gb(a.ram)}</span><input type="checkbox" checked={picked.has(a.exe)} onchange={() => tick(a.exe)} /></span>
        </label>
      {:else}
        <p class="hint">No heavy app is sitting unused right now.</p>
      {/each}
    {/if}
    <div class="field"><span></span>
      <span class="row">
        <button class="act" onclick={find} disabled={finding}>{finding ? 'Looking…' : idle ? 'Look again' : 'Find apps I\'m not using'}</button>
        {#if idle?.length}<button class="btn" onclick={closePicked} disabled={!picked.size || closing}><X size={14} /> {closing ? 'Closing…' : `Close ${picked.size || ''} selected`}</button>{/if}
      </span>
    </div>
    {#if closeMsg}<p class="msg">{closeMsg}</p>{/if}
  </div>
{/if}

<style>
  .compact { display: flex; align-items: center; gap: 10px; margin: 10px 0 2px; }
  /* The popup window has no shared .btn style (it lives in Settings). */
  .compact .btn { display: inline-flex; align-items: center; gap: 6px; flex: none; padding: 6px 12px; font-size: 12px;
                  border-radius: calc(9px * var(--rs)); background: var(--accent); color: var(--on-accent); }
  .compact .btn:disabled { opacity: 0.5; }
  .small { font-size: 11px; }
  .dim { color: var(--fg-dim); }
  .top { margin: 12px 0 4px; }
  .field small { display: block; font-weight: 400; font-size: 12px; margin-top: 2px; }
  .names { width: 220px; }
  .row { display: inline-flex; align-items: center; gap: 10px; }
  .act { display: inline-flex; align-items: center; gap: 5px; padding: 6px 12px; font-size: 12.5px; border-radius: calc(9px * var(--rs)); background: var(--pill); color: var(--fg); }
  .act:hover:not(:disabled) { background: var(--pill-hover); }
  .num { font-variant-numeric: tabular-nums; font-size: 13px; }
  .result { display: flex; flex-direction: column; gap: 6px; padding: 4px 0 14px; font-size: 13px; }
  .result b { font-weight: 600; }
  table { border-collapse: collapse; font-size: 12px; max-width: 320px; }
  td { padding: 3px 0; border-top: 1px solid var(--line-soft); font-variant-numeric: tabular-nums; }
  td + td { text-align: right; padding-left: 16px; }
  .msg { margin: 0 0 12px; padding: 7px 10px; border-radius: calc(8px * var(--rs)); font-size: 12.5px; background: var(--pill); }
  .bad { background: color-mix(in srgb, #ff453a 18%, transparent); }
</style>
