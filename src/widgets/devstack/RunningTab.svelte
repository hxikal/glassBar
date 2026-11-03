<script lang="ts">
  // Programs listening on localhost: dev servers first (Vite, Next.js, artisan serve…), the
  // rest (apps that keep a local port open) behind "Show all". Stop ends the npm chain too.
  import { FolderOpen, RefreshCw, Square } from '@lucide/svelte';
  import { ipc } from '../../lib/ipc';
  import type { DevStackSettings } from './index';
  import { poll, remember, remembered, shortDir } from './live.svelte';

  let { settings, report }: { settings: DevStackSettings; report: (e: unknown) => void } = $props();
  const list = poll(() => ipc.devPorts(settings.root, settings.projectDirs ?? []), 3_000);
  let all = $state(remembered('devstack.all', 'no', ['yes', 'no']) === 'yes');
  let busy = $state<number[]>([]);
  const dev = $derived((list.v ?? []).filter((l) => l.dev));
  const rest = $derived((list.v ?? []).filter((l) => !l.dev));
  const shown = $derived(all ? [...dev, ...rest] : dev);

  const setAll = (v: boolean) => { all = v; remember('devstack.all', v ? 'yes' : 'no'); };
  const open = (target: string) => ipc.appLaunch(target).catch(report);
  async function stop(pid: number) {
    busy = [...busy, pid]; report('');
    try { await ipc.devStopPid(pid); } catch (e) { report(e); }
    setTimeout(async () => { await list.load(); busy = busy.filter((b) => b !== pid); }, 1200);
  }
</script>

<div class="gtitle">
  <span class="dim">Listening on localhost · {shown.length}</span>
  {#if rest.length}
    <button class="link" onclick={() => setAll(!all)}>{all ? 'Dev servers only' : `Show all (+${rest.length})`}</button>
  {/if}
</div>

{#if !list.v}
  <p class="empty">{list.err || 'Looking…'}</p>
{:else}
  <div class="group">
    {#each shown as l (l.pid)}
      {@const b = busy.includes(l.pid)}
      <div class="svc on">
        <i class="dot"></i>
        <div class="col">
          <span class="top">
            <b class="ellipsis" title={l.cmd || l.name}>{l.label}</b>
            {#each l.ports.slice(0, 3) as p (p)}
              <button class="chip" onclick={() => open(`http://localhost:${p}`)} title="Open http://localhost:{p}">:{p}</button>
            {/each}
            {#if l.ports.length > 3}<span class="dim small">+{l.ports.length - 3}</span>{/if}
          </span>
          <span class="dim small ellipsis" title={l.dir}>{l.dir ? shortDir(l.dir) : l.name} · pid {l.pid}</span>
        </div>
        {#if l.dir}<button class="mini" onclick={() => open(l.dir)} aria-label="Open its folder" title="Open the folder"><FolderOpen size={13} /></button>{/if}
        <button class="act stop" disabled={b || l.db} onclick={() => stop(l.pid)} aria-label="Stop {l.label}"
                title={l.db ? 'A database: stop it from Services so no data is lost' : `Stop ${l.label}`}>
          {#if b}<RefreshCw size={13} class="spin" />{:else}<Square size={13} />{/if}
        </button>
      </div>
    {:else}
      <p class="empty">No dev servers running. Start one from Projects, or run <code>npm run dev</code>.</p>
    {/each}
  </div>
{/if}

<style>
  .top { display: flex; align-items: center; gap: 4px; min-width: 0; }
  .top b { min-width: 0; flex: 0 1 auto; }
  code { font-size: 11px; }
</style>
