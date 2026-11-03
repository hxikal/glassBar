<script lang="ts">
  import { ipc } from '../../lib/ipc';
  import { ADHANS } from '../../lib/sound';
  import LocationTag from '../../lib/LocationTag.svelte';
  import type { SettingsProps } from '../types';
  import type { SolatSettings } from './index';

  let { settings, save }: SettingsProps<SolatSettings> = $props();
  let zones = $state<[string, string, string][]>([]);
  ipc.solatZones().then((z) => (zones = z));
</script>

<label>Zone
  <select value={settings.zone} onchange={(e) => save({ zone: e.currentTarget.value })}>
    <option value="auto">Use shared location</option>
    {#each zones as [code, state, district] (code)}<option value={code}>{code} – {district} ({state})</option>{/each}
  </select>
</label>
{#if settings.zone === 'auto'}<div>Shared location: <LocationTag /></div>{/if}
<label><input type="checkbox" checked={settings.showHijri !== false} onchange={(e) => save({ showHijri: e.currentTarget.checked })} /> Show the Hijri date in the panel</label>
<label><input type="checkbox" checked={!!settings.hijriOnDock} onchange={(e) => save({ hijriOnDock: e.currentTarget.checked })} /> Hijri date on the large prayer pill</label>
<label><input type="checkbox" checked={settings.showTasbih !== false} onchange={(e) => save({ showTasbih: e.currentTarget.checked })} /> Tasbih counter in the panel</label>
<label><input type="checkbox" checked={settings.tasbihOnDock !== false} onchange={(e) => save({ tasbihOnDock: e.currentTarget.checked })} /> Tasbih counter beside the prayer pill</label>
<label><input type="checkbox" checked={settings.showImsak} onchange={(e) => save({ showImsak: e.currentTarget.checked })} /> Show Imsak</label>
<label><input type="checkbox" checked={settings.alerts} onchange={(e) => save({ alerts: e.currentTarget.checked })} /> Alert at each prayer time</label>
<label>At prayer time play
  <select value={settings.azanSound} disabled={!settings.alerts} onchange={(e) => save({ azanSound: e.currentTarget.value as SolatSettings['azanSound'] })}>
    <option value="short">Short adzan (opening)</option>
    <option value="full">Full adzan</option>
    <option value="chime">Normal alert sound</option>
  </select>
  <button class="btn" disabled={!settings.alerts} onclick={() => ipc.solatTestSound(settings)}>Test</button>
</label>
{#if settings.azanSound !== 'chime'}
<label>Adzan voice
  <select value={settings.azanVoice ?? 'madina'} disabled={!settings.alerts || !!settings.azanFile}
          onchange={(e) => save({ azanVoice: e.currentTarget.value as SolatSettings['azanVoice'] })}>
    {#each Object.entries(ADHANS) as [k, a] (k)}<option value={k}>{a.label}</option>{/each}
  </select>
</label>
<label>Adzan volume {settings.azanVolume ?? 100}%
  <input type="range" class="slider" min="5" max="100" step="5" value={settings.azanVolume ?? 100} disabled={!settings.alerts}
         style:--fill="{settings.azanVolume ?? 100}%" onchange={(e) => save({ azanVolume: +e.currentTarget.value })} />
</label>
<label><input type="checkbox" checked={settings.pauseMusic !== false} disabled={!settings.alerts} onchange={(e) => save({ pauseMusic: e.currentTarget.checked })} /> Pause music during adzan (resumes after)</label>
<label>Own adzan recording (MP3/WAV, optional, replaces the voice)
  <input style="flex:1" placeholder="C:\Users\me\Music\azan.mp3" value={settings.azanFile}
         onchange={(e) => save({ azanFile: e.currentTarget.value.trim().replace(/^"|"$/g, '') })} />
</label>
{/if}
<p class="dim small">Press ✓ on the prayer pill to stop the adzan.</p>
