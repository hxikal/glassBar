<script lang="ts">
  // JS projects (package.json with a dev/start script) with Start/Stop, then the .test sites.
  // Start opens a minimized console running `npm run dev`, so its logs stay readable.
  import { ExternalLink, FolderOpen, Play, RefreshCw, Square, SquareTerminal } from '@lucide/svelte';
  import { ipc, type DevProject } from '../../lib/ipc';
  import type { DevStackSettings } from './index';
  import { liveStack, poll } from './live.svelte';

  let { settings, report }: { settings: DevStackSettings; report: (e: unknown) => void } = $props();
  const projects = poll(() => ipc.devProjects(settings.root, settings.projectDirs ?? []), 3_000);
  const stack = liveStack(() => settings.root, 10_000);
  let busy = $state<string[]>([]);
  let filter = $state('');
  const match = (n: string) => n.toLowerCase().includes(filter.trim().toLowerCase());
  const js = $derived((projects.v ?? []).filter((p) => match(p.name)).sort((a, b) => Number(b.running) - Number(a.running)));
  const sites = $derived((stack.st?.sites ?? []).filter(match));
  const total = $derived((projects.v?.length ?? 0) + (stack.st?.sites.length ?? 0));

  const open = (target: string) => ipc.appLaunch(target).catch(report);
  const terminal = (dir: string) => ipc.devOpenTool(settings.root, 'terminal', dir).catch(report);

  /** Start/stop, spinning until the state flips (a dev server can take a while). */
  async function toggle(p: DevProject) {
    const want = !p.running;
    busy = [...busy, p.dir]; report('');
    try { await ipc.devProject(p.dir, want); } catch (e) { report(e); busy = busy.filter((b) => b !== p.dir); return; }
    for (let i = 0; i < 20; i++) {
      await new Promise((r) => setTimeout(r, 1000));
      await projects.load();
      if (projects.v?.find((x) => x.dir === p.dir)?.running === want) break;
    }
    busy = busy.filter((b) => b !== p.dir);
  }
</script>

{#if total > 8}<input class="search" placeholder="Filter projects and sites…" bind:value={filter} />{/if}

<div class="gtitle"><span class="dim">Node projects · {projects.v?.length ?? '…'}</span></div>
{#if !projects.v}
  <p class="empty">{projects.err || 'Looking…'}</p>
{:else}
  <div class="group">
    {#each js as p (p.dir)}
      {@const b = busy.includes(p.dir)}
      <div class="svc" class:on={p.running}>
        <i class="dot"></i>
        <div class="col">
          <span class="top">
            <b class="ellipsis" title={p.dir}>{p.name}</b>
            {#each p.ports.slice(0, 2) as port (port)}
              <button class="chip" onclick={() => open(`http://localhost:${port}`)} title="Open http://localhost:{port}">:{port}</button>
            {/each}
          </span>
          <span class="dim small ellipsis">{p.running ? (p.label || 'Starting…') : `${p.pm} run ${p.script}`}</span>
        </div>
        <button class="mini" onclick={() => terminal(p.dir)} aria-label="Terminal in {p.name}" title="Open a terminal here"><SquareTerminal size={13} /></button>
        <button class="mini" onclick={() => open(p.dir)} aria-label="Open the {p.name} folder" title="Open the folder"><FolderOpen size={13} /></button>
        <button class="act" class:stop={p.running} disabled={b} onclick={() => toggle(p)}
                aria-label="{p.running ? 'Stop' : 'Start'} {p.name}" title={p.running ? `Stop ${p.name}` : `${p.pm} run ${p.script}`}>
          {#if b}<RefreshCw size={13} class="spin" />{:else if p.running}<Square size={13} />{:else}<Play size={13} />{/if}
        </button>
      </div>
    {:else}
      <p class="empty">{filter ? 'No match.' : 'No folder with a package.json dev, serve or start script. Add project folders in Settings › Widgets › Dev Stack.'}</p>
    {/each}
  </div>
{/if}

{#if stack.st?.found}
  <div class="gtitle"><span class="dim">Sites · {stack.st.sites.length}</span><span class="dim small">http://&lt;folder&gt;.test</span></div>
  {#each sites as name (name)}
    <div class="site">
      <button class="sname ellipsis" onclick={() => open(`http://${name}.test`)} title="Open http://{name}.test">{name}<span class="dim">.test</span></button>
      <button class="mini" onclick={() => open(`http://${name}.test`)} aria-label="Open {name} in the browser" title="Open in the browser"><ExternalLink size={13} /></button>
      <button class="mini" onclick={() => terminal(`${stack.st!.www}\\${name}`)} aria-label="Terminal in {name}" title="Open a terminal here"><SquareTerminal size={13} /></button>
      <button class="mini" onclick={() => open(`${stack.st!.www}\\${name}`)} aria-label="Open the {name} folder" title="Open the folder"><FolderOpen size={13} /></button>
    </div>
  {:else}
    <p class="empty">{filter ? 'No match.' : `No sites in ${stack.st.www}.`}</p>
  {/each}
{/if}

<style>
  .search { width: 100%; box-sizing: border-box; margin-bottom: 8px; padding: 6px 10px; border-radius: calc(8px * var(--rs)); border: 1px solid var(--line-soft); background: var(--pill); outline: none; font-size: 12px; }
  .top { display: flex; align-items: center; gap: 4px; min-width: 0; }
  .site { display: flex; align-items: center; gap: 2px; border-radius: var(--r-inner); }
  .site:hover { background: var(--pill-hover); }
  .sname { flex: 1; min-width: 0; text-align: left; padding: 6px 8px; font-size: 12px; }
</style>
