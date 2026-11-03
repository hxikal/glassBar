<script lang="ts">
  import type { SettingsProps } from '../types';
  import type { DevStackSettings } from './index';

  let { settings, save }: SettingsProps<DevStackSettings> = $props();
</script>

<label>Laragon folder <input value={settings.root} placeholder="C:\laragon" onchange={(e) => save({ root: e.currentTarget.value.trim() || 'C:\\laragon' })} /></label>
<p class="dim" style="font-size:12px;margin:0">The folder that has laragon.exe. Services and sites are read from there.</p>
