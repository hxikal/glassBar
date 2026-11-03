<script lang="ts">
  // Small dropdown under the AI Usage header. Every change saves at once; click outside or
  // Esc closes it (Esc here doesn't also close the panel).
  import OptionsPopover from '../../lib/OptionsPopover.svelte';
  import type { AiUsageSettings } from './index';

  let { settings, labels, gear, save, close }: {
    settings: AiUsageSettings; labels: string[]; gear?: HTMLElement;
    save: (p: Partial<AiUsageSettings>) => void; close: () => void;
  } = $props();
  let box = $state<HTMLElement>();
  const hidden = $derived(settings.hidden ?? []);
  // Before limits load, still offer the usual three.
  const names = $derived(labels.length ? labels : ['Session', 'Weekly', 'Fable']);
  const values = $derived({
    show: settings.show, layout: settings.layout, track: settings.track ? 'on' : 'off',
    ...Object.fromEntries(names.map((l) => [`lim:${l}`, !hidden.includes(l)])),
  });
  function apply(p: Record<string, unknown>) {
    const lims = Object.entries(p).filter(([k]) => k.startsWith('lim:'));
    if (lims.length) save({ hidden: [...hidden.filter((h) => !names.includes(h)), ...lims.filter(([, on]) => !on).map(([k]) => k.slice(4))] });
    else if ('track' in p) save({ track: p.track === 'on' });
    else save(p as Partial<AiUsageSettings>);
  }
  const outside = (e: PointerEvent) => { const t = e.target as Node; if (!box?.contains(t) && !gear?.contains(t)) close(); };
  const esc = (e: KeyboardEvent) => { if (e.key === 'Escape') { e.stopPropagation(); close(); } };
</script>

<svelte:window onpointerdown={outside} onkeydowncapture={esc} />

<div class="qs" bind:this={box} role="dialog" aria-label="Quick settings">
  <OptionsPopover title="Quick settings" {values} save={apply} options={[
    { type: 'choice', key: 'show', label: 'Usage display', choices: [{ value: 'used', label: 'Used' }, { value: 'left', label: 'Left' }] },
    { type: 'checks', label: 'Show limits', min: 1, items: names.map((l) => ({ key: `lim:${l}`, label: l })) },
    { type: 'choice', key: 'layout', label: 'Bar layout', choices: [{ value: 'stacked', label: 'Stacked' }, { value: 'row', label: 'One row' }] },
    { type: 'choice', key: 'track', label: 'Background track', choices: [{ value: 'on', label: 'On' }, { value: 'off', label: 'Off' }] },
  ]} />
</div>

<style>
  .qs {
    position: absolute; right: 12px; top: 48px; z-index: 5; width: 300px; padding: 10px 10px 0;
    border-radius: var(--radius-card); background: var(--menu-bg); border: 1px solid var(--line-soft);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3); backdrop-filter: blur(16px);
  }
  .qs :global(.opts) { border-bottom: 0; margin-bottom: 0; }
  .qs :global(.checks) { flex-wrap: wrap; justify-content: flex-end; }
</style>
