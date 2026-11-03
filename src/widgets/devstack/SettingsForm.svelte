<script lang="ts">
  import type { SettingsProps } from '../types';
  import type { DevStackSettings } from './index';

  let { settings, save }: SettingsProps<DevStackSettings> = $props();
  const dirs = (text: string) => text.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
</script>

<label>Laragon folder <input value={settings.root} placeholder="C:\laragon" onchange={(e) => save({ root: e.currentTarget.value.trim() || 'C:\\laragon' })} /></label>
<p class="dim" style="font-size:12px;margin:0">The folder that has laragon.exe. Services and sites are read from there.</p>

<label>More project folders
  <textarea rows="3" placeholder={'D:\\projects\nC:\\Users\\me\\code'} value={(settings.projectDirs ?? []).join('\n')}
            onchange={(e) => save({ projectDirs: dirs(e.currentTarget.value) })}></textarea>
</label>
<p class="dim" style="font-size:12px;margin:0">One per line. Projects with a package.json dev, serve or start script in them (and in www) get Start/Stop in the Projects tab.</p>
