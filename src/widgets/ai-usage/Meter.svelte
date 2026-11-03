<script lang="ts">
  // One limit's bar. Used: fills as usage grows. Left: a battery (case + terminal nub) whose
  // charge drains as usage grows. The colour always comes from usage (format.ts tone).
  import { shownPct, tone } from './format';

  let { usedPct, show, track = true, h = 10 }: { usedPct: number | null; show: 'left' | 'used'; track?: boolean; h?: number } = $props();
</script>

<span class="meter" class:battery={show === 'left'} class:track style:--h="{h}px">
  <i style:width="{usedPct == null ? 0 : shownPct(usedPct, show)}%" style:background={usedPct == null ? 'transparent' : tone(usedPct)}></i>
</span>

<style>
  .meter { position: relative; display: block; flex: 1; min-width: 0; height: var(--h); border-radius: calc(var(--h) / 2); }
  .track { background: var(--track); }
  .meter i { display: block; height: 100%; border-radius: inherit; transition: width 0.3s ease, background-color 0.3s; }
  .battery { border-radius: 3px; margin-right: 4px; }
  .battery i { border-radius: 2px; }
  .battery.track { outline: 1px solid color-mix(in srgb, currentColor 40%, transparent); outline-offset: 1px; }
  .battery::after {
    content: ''; position: absolute; right: -5px; top: 25%; width: 2px; height: 50%;
    border-radius: 0 1px 1px 0; background: color-mix(in srgb, currentColor 45%, transparent);
  }
</style>
