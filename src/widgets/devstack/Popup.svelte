<script lang="ts">
  // Dev Stack panel: Services (Laragon + Windows services, Docker) · Running (localhost
  // servers) · Projects (npm dev servers and .test sites) · Versions (PHP / Node / Python).
  import { RefreshCw, Rocket } from '@lucide/svelte';
  import PanelHeader from '../../lib/PanelHeader.svelte';
  import { ipc } from '../../lib/ipc';
  import type { PopupProps } from '../types';
  import type { DevStackSettings } from './index';
  import { remember, remembered } from './live.svelte';
  import ServicesTab from './ServicesTab.svelte';
  import RunningTab from './RunningTab.svelte';
  import ProjectsTab from './ProjectsTab.svelte';
  import VersionsTab from './VersionsTab.svelte';

  let { settings }: PopupProps<DevStackSettings> = $props();
  const TABS = ['services', 'running', 'projects', 'versions'] as const;
  const NAMES = { services: 'Services', running: 'Running', projects: 'Projects', versions: 'Versions' };
  let tab = $state(remembered('devstack.tab', 'services', TABS));
  let nonce = $state(0);
  let error = $state('');
  let note = $state('');
  const report = (e: unknown) => { error = e ? String(e) : ''; };
  const notify = (n: string) => { note = n; };
  const setTab = (t: (typeof TABS)[number]) => { tab = t; error = ''; note = ''; remember('devstack.tab', t); };
</script>

<div class="devstack">
  <PanelHeader title="Dev Stack">
    <button class="icon-btn" onclick={() => nonce++} aria-label="Refresh" title="Refresh"><RefreshCw size={15} /></button>
    <button class="icon-btn" onclick={() => ipc.devOpenLaragon(settings.root).catch(report)} aria-label="Open Laragon" title="Open Laragon"><Rocket size={15} /></button>
  </PanelHeader>

  <div class="seg" role="tablist">
    {#each TABS as t (t)}
      <button role="tab" aria-selected={tab === t} class:sel={tab === t} onclick={() => setTab(t)}>{NAMES[t]}</button>
    {/each}
  </div>

  {#if error}<p class="err small">{error}</p>{/if}
  {#if note}<p class="note small">{note}</p>{/if}

  {#key nonce}
    {#if tab === 'services'}<ServicesTab {settings} {report} />
    {:else if tab === 'running'}<RunningTab {settings} {report} />
    {:else if tab === 'projects'}<ProjectsTab {settings} {report} />
    {:else}<VersionsTab {settings} {report} {notify} />{/if}
  {/key}
</div>

<style>
  .seg { display: grid; grid-template-columns: repeat(4, 1fr); gap: 2px; padding: 3px; border-radius: calc(10px * var(--rs)); background: var(--pill); margin-bottom: 10px; }
  .seg button { padding: 6px 2px; border-radius: calc(8px * var(--rs)); font-size: 12px; }
  .seg .sel { background: var(--accent); color: var(--on-accent); }
  .err { color: #ff6b6b; margin: 0 4px 8px; }
  .note { margin: 0 4px 8px; opacity: 0.8; }

  /* Shared by the tabs. */
  .devstack :global(.small) { font-size: 11px; }
  .devstack :global(.group) { display: flex; flex-direction: column; gap: var(--s-1); }
  .devstack :global(.gtitle) { display: flex; align-items: center; justify-content: space-between; gap: 8px; font-size: 12px; padding: 0 4px 4px; margin-top: 12px; }
  .devstack :global(.gtitle:first-child) { margin-top: 0; }
  .devstack :global(.svc) { display: flex; align-items: center; gap: var(--s-2); min-height: 42px; padding: var(--s-1) var(--s-3); border-radius: var(--r-inner); background: var(--bg-2); border: 1px solid var(--border); }
  .devstack :global(.dot) { width: 9px; height: 9px; border-radius: 50%; flex: none; background: color-mix(in srgb, currentColor 30%, transparent); }
  .devstack :global(.on .dot) { background: #34c759; box-shadow: 0 0 6px #34c75999; }
  .devstack :global(.pending .dot) { background: #ffcc00; }
  .devstack :global(.col) { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .devstack :global(.col b) { font-weight: 500; }
  .devstack :global(.act) { width: 30px; height: 30px; flex: none; display: grid; place-items: center; border-radius: 50%; background: var(--pill); color: #34c759; }
  .devstack :global(.act.stop) { color: #ff6b6b; }
  .devstack :global(.act:hover:not(:disabled)) { background: var(--pill-hover); }
  .devstack :global(.act:disabled) { opacity: 0.45; }
  .devstack :global(.spin) { animation: devstack-spin 1s linear infinite; }
  .devstack :global(.mini) { width: 26px; height: 26px; flex: none; display: grid; place-items: center; border-radius: 50%; opacity: 0.7; }
  .devstack :global(.mini:hover) { opacity: 1; background: var(--pill); }
  .devstack :global(.link) { font-size: 11px; opacity: 0.75; padding: 2px 6px; border-radius: calc(6px * var(--rs)); }
  .devstack :global(.link:hover) { opacity: 1; background: var(--pill); }
  .devstack :global(.chip) { font-size: 11px; font-variant-numeric: tabular-nums; padding: 1px 6px; border-radius: calc(6px * var(--rs)); background: var(--pill); }
  .devstack :global(button.chip:hover) { background: var(--pill-hover); }
  .devstack :global(.empty) { font-size: 12px; opacity: 0.6; padding: 8px 4px; margin: 0; }
  .devstack :global(.warn) { display: flex; gap: 6px; align-items: flex-start; font-size: 11px; color: #ffb340; margin: 4px 4px 0; }
  @keyframes -global-devstack-spin { to { transform: rotate(360deg); } }
</style>
