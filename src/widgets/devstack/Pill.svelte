<script lang="ts">
  // Server icon + one dot per service Laragon uses (or that is running): green = running.
  import { Server } from '@lucide/svelte';
  import type { PillProps } from '../types';
  import type { DevStackSettings } from './index';
  import { liveStack } from './live.svelte';

  let { size, settings, vertical }: PillProps<DevStackSettings> = $props();
  const stack = liveStack(() => settings.root, 10_000);
  const shown = $derived((stack.st?.services ?? []).filter((s) => s.enabled || s.running));
  const up = $derived(shown.filter((s) => s.running).length);
  const tip = $derived(!stack.st ? 'Dev Stack' : !stack.st.found ? `Laragon not found in ${settings.root}`
    : shown.map((s) => `${s.name}: ${s.running ? 'running' : 'stopped'}`).join('\n') || 'No services');
</script>

<div class="pill dev" class:vertical title={tip}>
  <Server size={16} class={stack.st?.found === false ? 'dim' : ''} />
  {#if size !== 'small' && !vertical}<span>Dev</span>{/if}
  <span class="dots" class:col={vertical}>
    {#each shown as s (s.id)}<i class:on={s.running} aria-label="{s.name} {s.running ? 'running' : 'stopped'}"></i>{/each}
  </span>
  {#if size !== 'small' && !vertical && shown.length}<span class="dim n">{up}/{shown.length}</span>{/if}
</div>

<style>
  .dev { gap: 7px; }
  .vertical { flex-direction: column; gap: 4px; }
  .dots { display: flex; gap: 3px; }
  .dots.col { flex-direction: column; }
  i { width: 7px; height: 7px; border-radius: 50%; background: color-mix(in srgb, currentColor 30%, transparent); }
  i.on { background: #34c759; box-shadow: 0 0 5px #34c75999; }
  .n { font-size: 11px; font-variant-numeric: tabular-nums; }
</style>
