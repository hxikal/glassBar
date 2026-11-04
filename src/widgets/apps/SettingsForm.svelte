<script lang="ts">
  import { ArrowDown, ArrowUp, FolderOpen, X } from '@lucide/svelte';
  import type { SettingsProps } from '../types';
  import type { AppsSettings, Pin } from './index';
  import { keyOf, nameOf, withPin } from './groups';
  import { perf } from '../../lib/perf.svelte';
  import OptionsPopover from '../../lib/OptionsPopover.svelte';
  import { ipc, type InstalledApp, type PinTarget } from '../../lib/ipc';

  let { settings, save }: SettingsProps<AppsSettings> = $props();
  let path = $state('');
  let error = $state('');
  let installed = $state<InstalledApp[] | null>(null);
  let q = $state('');
  const shown = $derived((installed ?? []).filter((a) => a.name.toLowerCase().includes(q.toLowerCase())).slice(0, 60));
  const pinnedKeys = $derived(new Set(settings.pinned.map((p) => keyOf(p.path))));
  const pinnedIds = $derived(new Set(settings.pinned.map((p) => p.aumid).filter(Boolean)));
  const isPinned = (a: InstalledApp) => pinnedKeys.has(keyOf(a.path)) || (!!a.aumid && pinnedIds.has(a.aumid));

  function pin(t: PinTarget & { aumid?: string | null; name?: string }) {
    const pinned = withPin(settings.pinned, t);
    error = pinned ? '' : `${t.name ?? nameOf(t.path)} is already pinned.`;
    if (pinned) save({ pinned });
  }
  async function addTyped() {
    const p = path.trim().replace(/^"|"$/g, '');
    if (!p) return;
    try { pin(await ipc.appPinTarget(p)); path = ''; } catch (e) { error = String(e); }
  }
  async function browse() {
    try { const t = await ipc.appBrowse(); if (t) pin(t); } catch (e) { error = String(e); }
  }
  function move(i: number, by: number) {
    const pins: Pin[] = [...settings.pinned];
    const [p] = pins.splice(i, 1);
    pins.splice(i + by, 0, p);
    // Keep the bar's drag order in step with the list.
    save({ pinned: pins, order: [...pins.map((x) => keyOf(x.path)), ...settings.order.filter((k) => !pins.some((x) => keyOf(x.path) === k))] });
  }
</script>

<OptionsPopover title="Start & Search" values={settings} save={(p) => save(p as Partial<AppsSettings>)} options={[
  { type: 'choice', key: 'startPlacement', label: 'Placement', choices: [{ value: 'inline', label: 'Left of apps' }, { value: 'after', label: 'Right of apps' }, { value: 'separate', label: 'Own widget' }] },
  { type: 'choice', key: 'searchStyle', label: 'Search', choices: [{ value: 'icon', label: 'Icon' }, { value: 'bar', label: 'Search bar' }] },
  { type: 'choice', key: 'searchOpens', label: 'Search opens', choices: [{ value: 'glassbar', label: 'GlassBar', hint: 'Calculator, units, currency and apps as you type' }, { value: 'windows', label: 'Windows', hint: 'Windows Search (Win+S)' }] },
  { type: 'choice', key: 'startColor', label: 'Windows logo colour', menu: true, choices: [
    { value: 'blue', label: 'Windows blue' },
    { value: 'accent', label: 'Accent colour', hint: 'Follows Settings > Appearance' },
    { value: 'text', label: 'Same as text', hint: 'White on a dark dock, dark on a light one' },
    { value: 'classic', label: 'Classic four colours' },
    { value: 'custom', label: 'Custom…' }] },
  ...(settings.startColor === 'custom' ? [{ type: 'color' as const, key: 'startCustom', label: 'Logo colour' }] : []),
]} />
<label>Icons shown {settings.maxIcons ? settings.maxIcons : 'all'}
  <input type="range" class="slider" min="0" max="20" step="1" value={settings.maxIcons ?? 0} style:--fill="{((settings.maxIcons ?? 0) / 20) * 100}%"
         onchange={(e) => save({ maxIcons: +e.currentTarget.value })} />
</label>
<p class="dim small">The rest show as "+N"; hover the launcher to see them all. 0 = show all.</p>
<label><input type="checkbox" checked={settings.showTitles} onchange={(e) => save({ showTitles: e.currentTarget.checked })} /> Show active window title (large size)</label>

<h4>Dock effects</h4>
<label><input type="checkbox" checked={settings.magnify ?? false} onchange={(e) => save({ magnify: e.currentTarget.checked })} /> Magnification</label>
{#if settings.magnify}
  {@const k = settings.magnifyScale ?? 1.6}
  <label>Size {k.toFixed(1)}×
    <input type="range" class="slider" min="1.2" max="2" step="0.1" value={k} style:--fill="{((k - 1.2) / 0.8) * 100}%"
           onchange={(e) => save({ magnifyScale: +e.currentTarget.value })} />
  </label>
{/if}
<label>Minimize windows using
  <select value={settings.minimizeEffect ?? 'none'} onchange={(e) => save({ minimizeEffect: e.currentTarget.value as AppsSettings['minimizeEffect'] })}>
    <option value="genie">Genie</option><option value="scale">Scale</option><option value="suck">Suck</option><option value="none">Off</option>
  </select>
</label>
<label><input type="checkbox" checked={settings.bounce ?? false} onchange={(e) => save({ bounce: e.currentTarget.checked })} /> Animate opening applications</label>
{#if perf.mode === 'low'}<p class="dim small">Low performance mode is on: minimizing always uses Scale, the magnified icons don't glide, and an opening app's dot blinks instead of its icon bouncing.</p>{/if}

<p class="dim">Pinned apps stay on the bar even when closed. Drag icons on the bar to reorder them, or right-click an icon to pin/unpin.</p>
<ul>
  {#each settings.pinned as p, i (p.path + i)}
    <li>
      <span class="ellipsis" title={p.path}><b>{p.name ?? nameOf(p.path)}</b> <span class="dim">{p.path}{p.args ? ` ${p.args}` : ''}</span></span>
      <span class="acts">
        <button class="icon-btn" aria-label="Move up" disabled={i === 0} onclick={() => move(i, -1)}><ArrowUp size={14} /></button>
        <button class="icon-btn" aria-label="Move down" disabled={i === settings.pinned.length - 1} onclick={() => move(i, 1)}><ArrowDown size={14} /></button>
        <button class="icon-btn" aria-label="Unpin" onclick={() => save({ pinned: settings.pinned.filter((_, j) => j !== i) })}><X size={14} /></button>
      </span>
    </li>
  {/each}
</ul>

<div class="row">
  <input placeholder="Paste a path to an .exe or shortcut" bind:value={path} onkeydown={(e) => e.key === 'Enter' && addTyped()} />
  <button class="btn" onclick={addTyped}>Pin</button>
  <button class="btn" onclick={browse} title="Browse for a program"><FolderOpen size={14} /> Browse…</button>
</div>
{#if error}<p class="err">{error}</p>{/if}

<details ontoggle={(e) => { if ((e.currentTarget as HTMLDetailsElement).open && !installed) ipc.appsInstalled().then((a) => (installed = a)); }}>
  <summary>Pin from installed apps</summary>
  <input placeholder="Search apps…" bind:value={q} />
  {#if !installed}<p class="dim">Reading the Start menu…</p>{/if}
  <div class="inst">
    {#each shown as a (a.path + a.args)}
      <button class="ia" disabled={isPinned(a)} onclick={() => pin(a)} title={a.path}>
        <span class="ellipsis">{a.name}</span><span class="dim">{isPinned(a) ? 'Pinned' : 'Pin'}</span>
      </button>
    {/each}
  </div>
</details>

<style>
  ul { list-style: none; padding: 0; margin: 4px 0; }
  li { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .acts { display: flex; gap: 2px; flex: none; }
  .row { display: flex; gap: 6px; }
  .row input { flex: 1; }
  .err { color: #ff453a; font-size: 12px; }
  summary { cursor: pointer; margin: 10px 0 6px; }
  .inst { max-height: 220px; overflow: auto; display: flex; flex-direction: column; gap: 2px; margin-top: 6px; }
  .ia { display: flex; justify-content: space-between; gap: 8px; padding: 6px 8px; border-radius: calc(8px * var(--rs)); text-align: left; }
  .ia:hover:not(:disabled) { background: var(--pill); }
  .ia:disabled { opacity: 0.5; }
  .small { font-size: 11px; }
  h4 { margin: 12px 0 4px; font-size: 12px; font-weight: 600; }
</style>
