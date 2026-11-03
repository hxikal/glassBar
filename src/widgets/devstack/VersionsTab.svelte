<script lang="ts">
  // PHP / Node / Python versions. PHP switches like Laragon's menu (web server + terminals);
  // Node through nvm when installed (Windows asks for admin), else Laragon's own Node.
  // Also says what a new terminal really runs, and what gets in the way.
  import { Database, FileCog, TriangleAlert } from '@lucide/svelte';
  import { ipc, type DevKind } from '../../lib/ipc';
  import type { DevStackSettings } from './index';
  import { poll, prettyVersion } from './live.svelte';

  let { settings, report, notify }: { settings: DevStackSettings; report: (e: unknown) => void; notify: (n: string) => void } = $props();
  const vers = poll(() => ipc.devVersions(settings.root), 10_000);
  let busy = $state('');

  async function run(kind: string, job: () => Promise<string | void>) {
    busy = kind; report(''); notify('');
    try { const n = await job(); if (n) notify(n); } catch (e) { report(e); }
    await vers.load();
    busy = '';
  }
  const laragon = (kind: 'php' | 'nodejs' | 'python', v: string) => run(kind, () => ipc.devSwitch(settings.root, kind, v));
  const nvm = (v: string) => run('node', () => ipc.devNvmUse(v));
</script>

{#snippet picker(kind: string, k: DevKind, pick: (v: string) => void, pretty = true)}
  <select class="pick" value={k.current} disabled={busy !== '' || k.installed.length < 2} onchange={(e) => pick(e.currentTarget.value)}>
    {#if !k.installed.includes(k.current)}<option value={k.current}>{k.current || '—'}</option>{/if}
    {#each k.installed as v (v)}<option value={v}>{pretty ? prettyVersion(v) : v}</option>{/each}
  </select>
  {#if busy === kind}<span class="dim small">Switching…</span>{/if}
{/snippet}

{#if !vers.v}
  <p class="empty">{vers.err || 'Reading…'}</p>
{:else}
  {@const v = vers.v}
  <div class="gtitle"><span class="dim">PHP</span><span class="dim small">web server + new terminals</span></div>
  <div class="card">
    <div class="line">
      {#if v.php.installed.length}{@render picker('php', v.php, (x) => laragon('php', x))}{:else}<span class="dim small">No PHP in Laragon's bin\php</span>{/if}
      <button class="mini" onclick={() => ipc.devOpenTool(settings.root, 'phpini').catch(report)} aria-label="Edit php.ini" title="Edit php.ini"><FileCog size={14} /></button>
    </div>
    {#if v.phpTerminal}
      <span class="dim small">Terminals run PHP {prettyVersion(v.phpTerminal.version)}</span>
      {#if v.phpTerminal.shadowed}
        <p class="warn"><TriangleAlert size={13} /> <span>{v.phpTerminal.path} comes first in your PATH, so terminals don't use the version picked here.</span></p>
      {/if}
    {/if}
  </div>

  <div class="gtitle"><span class="dim">Node</span><span class="dim small">{v.nvm ? 'nvm · asks for admin' : 'Laragon'}</span></div>
  <div class="card">
    <div class="line">
      {#if v.nvm}{@render picker('node', v.nvm, nvm, false)}
      {:else if v.nodejs.installed.length}{@render picker('nodejs', v.nodejs, (x) => laragon('nodejs', x))}
      {:else}<span class="dim small">No nvm, and no Node in Laragon's bin\nodejs</span>{/if}
    </div>
    {#if v.nodeTerminal}
      <span class="dim small ellipsis" title={v.nodeTerminal.path}>Terminals run Node {v.nodeTerminal.version || '?'}</span>
      {#if v.nodeTerminal.shadowed}
        <p class="warn"><TriangleAlert size={13} /> <span>{v.nodeTerminal.path} comes first in your PATH, so terminals use its Node, not the one picked here. Remove it from PATH to switch with {v.nvm ? 'nvm' : 'Laragon'}.</span></p>
      {/if}
    {:else}
      <span class="dim small">No node on PATH</span>
    {/if}
  </div>

  {#if v.python.installed.length}
    <div class="gtitle"><span class="dim">Python</span></div>
    <div class="card">
      <div class="line">{@render picker('python', v.python, (x) => laragon('python', x))}</div>
    </div>
  {/if}

  <div class="tools">
    <button class="tile" onclick={() => ipc.devOpenTool(settings.root, 'heidisql').catch(report)}><Database size={14} /> HeidiSQL</button>
  </div>
  {#if v.laragonRunning}<p class="dim small hint">Laragon is open. After a switch it may need Menu › Reload, or it can switch back.</p>{/if}
  <p class="dim small hint">Each switch saves a backup in {settings.root}\usr\glassbar-backup.</p>
{/if}

<style>
  .card { display: flex; flex-direction: column; gap: 4px; padding: var(--s-2) var(--s-3); border-radius: var(--r-inner); background: var(--bg-2); border: 1px solid var(--border); }
  .line { display: flex; align-items: center; gap: 8px; }
  .pick { flex: 1; min-width: 0; font-size: 12px; padding: 5px 8px; border-radius: calc(8px * var(--rs)); border: 1px solid var(--line-soft); background: var(--pill); color: inherit; }
  .tools { display: flex; gap: var(--s-2); margin-top: 12px; }
  .tools .tile { flex-direction: row; align-items: center; padding: 8px 12px; }
  .hint { margin: 8px 4px 0; }
</style>
