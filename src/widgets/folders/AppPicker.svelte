<script lang="ts">
  // Add apps to a folder from the installed apps (the Start menu's list): type to filter,
  // click to add -- it stays open so several go in one after another. Apps already in the
  // folder show a tick. "Browse…" picks any program or shortcut file instead.
  import { Check, Search } from '@lucide/svelte';
  import { ipc, type InstalledApp } from '../../lib/ipc';
  import type { Pin } from '../apps/index';
  import { iconOf } from './appIcons.svelte';
  import { pinId } from './folders';

  let { have, add, done }: { have: Pin[]; add: (p: Pin) => void; done: () => void } = $props();
  let all = $state<InstalledApp[] | null>(null);
  let q = $state('');
  $effect(() => { ipc.appsInstalled().then((a) => (all = [...a].sort((x, y) => x.name.localeCompare(y.name)))).catch(() => (all = [])); });
  const inFolder = $derived(new Set(have.map(pinId)));
  const shown = $derived((all ?? []).filter((a) => !q.trim() || a.name.toLowerCase().includes(q.trim().toLowerCase())).slice(0, 60));
  const toPin = (a: InstalledApp): Pin => ({ path: a.path, args: a.args || undefined, aumid: a.aumid ?? undefined, name: a.name });
  async function browse() {
    const t = await ipc.appBrowse().catch(() => null);
    if (t) add({ path: t.path, args: t.args || undefined });
  }
</script>

<div class="picker">
  <label class="search">
    <Search size={14} />
    <!-- svelte-ignore a11y_autofocus -->
    <input bind:value={q} placeholder="Search apps" aria-label="Search apps" autofocus onkeydown={(e) => e.key === 'Escape' && done()} />
  </label>
  <div class="list">
    {#if all === null}
      <p class="dim hint">Loading apps…</p>
    {:else if !shown.length}
      <p class="dim hint">No app called “{q}”.</p>
    {/if}
    {#each shown as a (a.path + a.args)}
      {@const p = toPin(a)}
      {@const added = inFolder.has(pinId(p))}
      {@const src = iconOf(p)}
      <button class="row" class:added disabled={added} onclick={() => add(p)} title={added ? 'Already in this folder' : `Add ${a.name}`}>
        {#if src}<img src={src} alt="" width="20" height="20" draggable="false" />{:else}<i class="ph">{a.name[0]?.toUpperCase()}</i>{/if}
        <span class="nm ellipsis">{a.name}</span>
        {#if added}<Check size={14} />{/if}
      </button>
    {/each}
  </div>
  <div class="foot">
    <button class="link" onclick={browse}>Browse for a program…</button>
    <button class="btn-done" onclick={done}>Done</button>
  </div>
</div>

<style>
  .picker { display: flex; flex-direction: column; gap: 8px; animation: rise 0.18s var(--ease); }
  @keyframes rise { from { opacity: 0; transform: translateY(6px); } }
  .search { display: flex; align-items: center; gap: 8px; height: 36px; padding: 0 10px; border-radius: var(--r-inner); background: var(--bg-2, var(--pill)); color: var(--fg-dim); }
  .search:focus-within { box-shadow: inset 0 0 0 1.5px var(--accent); }
  .search input { flex: 1; min-width: 0; border: 0; outline: none; background: transparent; color: var(--fg); font: inherit; font-size: 13px; }
  .list { display: flex; flex-direction: column; gap: 1px; max-height: 220px; overflow-y: auto; }
  .row { display: flex; align-items: center; gap: 10px; height: 34px; padding: 0 8px; border-radius: var(--r-inner); text-align: left; transition: background 0.15s var(--ease); }
  .row:hover:not(:disabled) { background: var(--pill-hover); }
  .row:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .row.added { color: var(--fg-dim); }
  .row img { width: 20px; height: 20px; object-fit: contain; flex: none; }
  .ph { display: grid; place-items: center; width: 20px; height: 20px; flex: none; border-radius: 5px; font-size: 10px; font-style: normal; font-weight: 600; background: var(--accent); color: var(--on-accent); }
  .nm { flex: 1; font-size: 12.5px; }
  .hint { margin: 8px 4px; font-size: 11.5px; }
  .foot { display: flex; align-items: center; justify-content: space-between; }
  .link { color: var(--accent); font-size: 12px; }
  .btn-done { height: 30px; padding: 0 14px; border-radius: var(--r-chip); background: var(--accent); color: var(--on-accent); font-size: 12px; font-weight: 500; }
  .btn-done:active { transform: scale(0.96); }
  @media (prefers-reduced-motion: reduce) { .picker { animation: none; } }
</style>
