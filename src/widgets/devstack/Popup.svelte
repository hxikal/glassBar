<script lang="ts">
  // Services with start/stop, Start all / Stop all for the ones Laragon uses, and the sites
  // in www (each opens at http://<folder>.test, or as a folder).
  import { ExternalLink, FolderOpen, Play, RefreshCw, Rocket, Square } from '@lucide/svelte';
  import PanelHeader from '../../lib/PanelHeader.svelte';
  import { ipc, type DevService } from '../../lib/ipc';
  import type { PopupProps } from '../types';
  import type { DevStackSettings } from './index';
  import { liveStack } from './live.svelte';

  let { settings }: PopupProps<DevStackSettings> = $props();
  const stack = liveStack(() => settings.root, 3_000);
  let busy = $state<string[]>([]);
  let error = $state('');
  let filter = $state('');
  const sites = $derived((stack.st?.sites ?? []).filter((s) => s.toLowerCase().includes(filter.trim().toLowerCase())));
  const main = $derived((stack.st?.services ?? []).filter((s) => s.enabled));

  async function toggle(s: DevService, start = !s.running) {
    if (s.running === start) return;
    busy = [...busy, s.id]; error = '';
    try { await ipc.devToggle(settings.root, s.id, start); }
    catch (e) { error = String(e); }
    // Services take a moment to come up or go down.
    setTimeout(async () => { await stack.load(); busy = busy.filter((b) => b !== s.id); }, 1500);
  }
  const all = (start: boolean) => main.forEach((s) => toggle(s, start));
  const open = (target: string) => ipc.appLaunch(target).catch((e) => (error = String(e)));
</script>

<PanelHeader title="Dev Stack">
  <button class="icon-btn" onclick={() => stack.load()} aria-label="Refresh" title="Refresh"><RefreshCw size={15} /></button>
  <button class="icon-btn" onclick={() => ipc.devOpenLaragon(settings.root).catch((e) => (error = String(e)))} aria-label="Open Laragon" title="Open Laragon"><Rocket size={15} /></button>
</PanelHeader>

{#if !stack.st}
  <p class="dim small">Reading Laragon…</p>
{:else if !stack.st.found}
  <p class="dim small">Laragon isn't in <b>{settings.root}</b>. Set its folder in Settings › Widgets › Dev Stack.</p>
{:else}
  <div class="svcs">
    {#each stack.st.services as s (s.id)}
      {@const b = busy.includes(s.id)}
      <div class="svc" class:on={s.running}>
        <i class="dot"></i>
        <div class="col">
          <b>{s.name}</b>
          <span class="dim small ellipsis">{s.version} · port {s.port}{s.enabled ? '' : ' · not used by Laragon'}</span>
        </div>
        <button class="act" class:stop={s.running} disabled={b} onclick={() => toggle(s)}
                aria-label="{s.running ? 'Stop' : 'Start'} {s.name}" title="{s.running ? 'Stop' : 'Start'} {s.name}">
          {#if b}<RefreshCw size={13} class="spin" />{:else if s.running}<Square size={13} />{:else}<Play size={13} />{/if}
        </button>
      </div>
    {/each}
  </div>
  {#if main.length}
    <div class="row2">
      <button class="tile" onclick={() => all(true)} disabled={main.every((s) => s.running)}>Start all</button>
      <button class="tile" onclick={() => all(false)} disabled={main.every((s) => !s.running)}>Stop all</button>
    </div>
  {/if}
  {#if error}<p class="err small">{error}</p>{/if}

  <div class="sites">
    <div class="stitle">
      <span class="dim">Sites · {stack.st.sites.length}</span>
      {#if stack.st.sites.length > 6}<input class="search" placeholder="Filter…" bind:value={filter} />{/if}
    </div>
    {#each sites as name (name)}
      <div class="site">
        <button class="sname ellipsis" onclick={() => open(`http://${name}.test`)} title="Open http://{name}.test">{name}<span class="dim">.test</span></button>
        <button class="mini" onclick={() => open(`http://${name}.test`)} aria-label="Open {name} in the browser" title="Open in the browser"><ExternalLink size={13} /></button>
        <button class="mini" onclick={() => open(`${stack.st!.www}\\${name}`)} aria-label="Open the {name} folder" title="Open the folder"><FolderOpen size={13} /></button>
      </div>
    {:else}
      <p class="dim small">No sites in {stack.st.www}.</p>
    {/each}
  </div>
{/if}

<style>
  .small { font-size: 11px; }
  .svcs { display: flex; flex-direction: column; gap: var(--s-1); }
  .svc { display: flex; align-items: center; gap: var(--s-2); min-height: 42px; padding: var(--s-1) var(--s-3); border-radius: var(--r-inner); background: var(--bg-2); border: 1px solid var(--border); }
  .dot { width: 9px; height: 9px; border-radius: 50%; flex: none; background: color-mix(in srgb, currentColor 30%, transparent); }
  .on .dot { background: #34c759; box-shadow: 0 0 6px #34c75999; }
  .col { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .col b { font-weight: 500; }
  .act { width: 30px; height: 30px; display: grid; place-items: center; border-radius: 50%; background: var(--pill); color: #34c759; }
  .act.stop { color: #ff6b6b; }
  .act:hover:not(:disabled) { background: var(--pill-hover); }
  .act:disabled { opacity: 0.6; }
  .act :global(.spin) { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .row2 { display: grid; grid-template-columns: 1fr 1fr; gap: var(--s-2); margin-top: var(--s-2); }
  .err { color: #ff6b6b; margin: 6px 4px 0; }
  .sites { margin-top: 12px; }
  .stitle { display: flex; align-items: center; justify-content: space-between; gap: 8px; font-size: 12px; padding: 0 4px 4px; }
  .search { width: 130px; padding: 4px 8px; border-radius: calc(8px * var(--rs)); border: 1px solid var(--line-soft); background: var(--pill); outline: none; font-size: 12px; }
  .site { display: flex; align-items: center; gap: 2px; border-radius: var(--r-inner); }
  .site:hover { background: var(--pill-hover); }
  .sname { flex: 1; min-width: 0; text-align: left; padding: 6px 8px; font-size: 12px; }
  .mini { width: 26px; height: 26px; display: grid; place-items: center; border-radius: 50%; opacity: 0.7; }
  .mini:hover { opacity: 1; background: var(--pill); }
</style>
