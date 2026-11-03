<script lang="ts">
  import type { SettingsProps } from '../types';
  import type { AiUsageSettings } from './index';
  import { pillRows } from './format';
  import { liveLimits } from './limits.svelte';

  let { settings, save }: SettingsProps<AiUsageSettings> = $props();
  // Rows this account has (Session, Weekly, then per-model like Fable).
  const lim = liveLimits();
  const labels = $derived(pillRows(lim.l?.windows ?? []).map((r) => r.label));
  const hidden = $derived(settings.hidden ?? []);
  const shownCount = $derived(labels.filter((l) => !hidden.includes(l)).length);
  function toggle(label: string, on: boolean) {
    save({ hidden: on ? hidden.filter((h) => h !== label) : [...hidden, label] });
  }
</script>

<label>Show percent
  <select value={settings.show} onchange={(e) => save({ show: e.currentTarget.value as AiUsageSettings['show'] })}>
    <option value="used">Used (e.g. 86% used)</option>
    <option value="left">Left (e.g. 14% left)</option>
  </select>
</label>
<p class="dim" style="font-size:12px;margin-bottom:2px">Show on dock</p>
{#each labels as l (l)}
  {@const on = !hidden.includes(l)}
  <!-- The last ticked row can't be unticked: the dock always shows at least one. -->
  <label><input type="checkbox" checked={on} disabled={on && shownCount === 1} onchange={(e) => toggle(l, e.currentTarget.checked)} /> {l}{l === 'Session' ? ' (5-hour)' : ''}</label>
{/each}
<label>Graph range
  <select value={String(settings.days)} onchange={(e) => save({ days: +e.currentTarget.value })}>
    <option value="7">7 days</option>
    <option value="14">14 days</option>
    <option value="30">30 days</option>
  </select>
</label>
<p class="dim" style="font-size:12px">Limits use the Claude Code login on this PC (run <code>claude</code> once to sign in). The graph counts Claude Code tokens from local logs; claude.ai chats aren't included.</p>
