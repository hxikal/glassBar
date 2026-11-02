<script lang="ts">
  // One folder, opened from the dock: its name (click to rename; empty = no label) and its
  // apps in a 4-column grid. Click an app to open it (or switch to its window). Drag to
  // reorder; drag one out of the grid to take it out of the folder. + adds a program or
  // shortcut, and so does dropping one from Explorer onto the panel. With id ALL it lists every
  // folder first; picking one opens it here, with a way back.
  import { ChevronLeft, Plus } from '@lucide/svelte';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { ipc } from '../../lib/ipc';
  import { profile, updateWidgetSettings } from '../../lib/config.svelte';
  import { showMenu } from '../../lib/menu';
  import { nextWindow } from '../apps/groups';
  import type { Pin } from '../apps/index';
  import manifest, { type FoldersSettings } from './index';
  import { settingsOf } from '../types';
  import { ALL, addApps, isApp, moveApp, pinId, pinName, removeApp, replace, type Folder } from './folders';
  import { iconOf, liveWindows } from './appIcons.svelte';
  import AppPicker from './AppPicker.svelte';

  let { instance, id, close }: { instance: string; id: string; close: () => void } = $props();
  // With the widget's defaults merged in, like every other widget: a new Folders widget has
  // saved nothing yet, and reading the raw settings made its first folder look deleted.
  const inst = $derived(profile().widgets.find((w) => w.instance === instance));
  const settings = $derived(inst ? settingsOf<FoldersSettings>(manifest, inst) : undefined);
  let picked = $state<string | null>(null); // a folder opened from the All folders list
  const showing = $derived(picked ?? id);
  const folder = $derived(settings?.folders?.find((f) => f.id === showing));
  const save = (f: Folder) => settings && updateWidgetSettings(instance, { folders: replace(settings.folders, f) });
  const wins = liveWindows();

  function launch(p: Pin) {
    if (swallow) { swallow = false; return; }
    const ws = wins.of(p);
    const w = ws.length ? nextWindow({ key: '', path: p.path, name: '', windows: ws }) : undefined;
    if (w) ipc.appActivate(w.id); else ipc.appLaunch(p.path, p.args ?? '', p.aumid ?? null).catch(() => {});
    close();
  }
  // Add app: the installed apps, searchable, inside the panel (a file dialog would take
  // focus and close a click-opened panel).
  let picking = $state(false);
  const addOne = (p: Pin) => { if (folder) save(addApps(folder, [p])); };
  // A folder in the All folders list: open it, or delete it (a submenu as the confirmation,
  // like on the dock; the apps themselves are untouched).
  function folderMenu(e: MouseEvent, f: Folder) {
    e.preventDefault();
    showMenu([
      { text: 'Open', action: () => (picked = f.id) },
      '-',
      { text: 'Delete folder', items: [
        { text: `Delete ${f.name ? `“${f.name}”` : 'this folder'}${f.apps.length ? ` and its ${f.apps.length} shortcut${f.apps.length > 1 ? 's' : ''}` : ''}`,
          action: () => settings && updateWidgetSettings(instance, { folders: settings.folders.filter((x) => x.id !== f.id) }) },
        { text: 'Keep it', action: () => {} },
      ] },
    ]);
  }
  function menu(e: MouseEvent, p: Pin, i: number) {
    e.preventDefault();
    if (!folder) return;
    const f = folder;
    showMenu([
      { text: 'Open', action: () => launch(p) },
      { text: 'Show in folder', action: () => ipc.downloadsReveal(p.path).catch(() => {}) },
      '-',
      ...(i > 0 ? [{ text: 'Move left', action: () => save(moveApp(f, pinId(p), i - 1)) }] : []),
      ...(i < f.apps.length - 1 ? [{ text: 'Move right', action: () => save(moveApp(f, pinId(p), i + 1)) }] : []),
      { text: 'Take out of the folder', action: () => save(removeApp(f, pinId(p))) },
    ]);
  }

  // ---- Drag: hold or move to pick an app up; it takes the place of the nearest cell, the
  // others make room. Released outside the grid, it leaves the folder. Esc puts it back. ----
  let grid = $state<HTMLElement>();
  let dragKey = $state<string | null>(null);
  let order = $state<Pin[] | null>(null);
  let outside = $state(false);
  let swallow = false;
  const shown = $derived(order ?? folder?.apps ?? []);
  function down(e: PointerEvent, p: Pin) {
    if (e.button !== 0 || !folder) return;
    const start = { x: e.clientX, y: e.clientY };
    let picked = false;
    const pick = () => { picked = true; dragKey = pinId(p); order = [...folder!.apps]; };
    const timer = setTimeout(pick, 250);
    const move = (ev: PointerEvent) => {
      if (!picked && Math.hypot(ev.clientX - start.x, ev.clientY - start.y) > 6) { clearTimeout(timer); pick(); }
      if (!picked || !order || !grid) return;
      const g = grid.getBoundingClientRect();
      outside = ev.clientX < g.left - 24 || ev.clientX > g.right + 24 || ev.clientY < g.top - 24 || ev.clientY > g.bottom + 24;
      if (outside) return;
      const cells = [...grid.querySelectorAll<HTMLElement>('.app')].map((c) => { const r = c.getBoundingClientRect(); return Math.hypot(ev.clientX - (r.left + r.width / 2), ev.clientY - (r.top + r.height / 2)); });
      const to = cells.indexOf(Math.min(...cells));
      const from = order.findIndex((x) => pinId(x) === dragKey);
      if (to >= 0 && to !== from) order = moveApp({ id, name: '', apps: order }, dragKey!, to).apps;
    };
    const finish = (keep: boolean) => {
      clearTimeout(timer);
      removeEventListener('pointermove', move);
      removeEventListener('pointerup', up);
      removeEventListener('keydown', esc);
      if (picked && keep && folder) {
        swallow = !outside; // released outside: no click follows to swallow
        save(outside ? removeApp(folder, dragKey!) : { ...folder, apps: order ?? folder.apps });
      }
      dragKey = null; order = null; outside = false;
    };
    const up = () => finish(true);
    const esc = (k: KeyboardEvent) => { if (k.key === 'Escape') finish(false); };
    addEventListener('pointermove', move);
    addEventListener('pointerup', up);
    addEventListener('keydown', esc);
  }

  // Programs and shortcuts dropped from Explorer anywhere on the panel.
  let dropping = $state(false);
  $effect(() => {
    const off = getCurrentWebviewWindow().onDragDropEvent((e) => {
      const d = e.payload;
      if (d.type === 'enter' || d.type === 'over') { dropping = true; return; }
      dropping = false;
      if (d.type === 'drop' && folder) {
        const apps = d.paths.filter(isApp).map((path) => ({ path }));
        if (apps.length) save(addApps(folder, apps));
      }
    });
    return () => { off.then((f) => f()).catch(() => {}); };
  });
</script>

{#if showing === ALL}
  <div class="folderp">
    <p class="title">All folders</p>
    <div class="grid">
      {#each settings?.folders ?? [] as f (f.id)}
        <button class="app" onclick={() => (picked = f.id)} oncontextmenu={(e) => folderMenu(e, f)}
                title={(f.apps.map(pinName).join(', ') || 'Empty folder') + ' · right-click to delete'}>
          <span class="mini">
            {#each f.apps.slice(0, 4) as p (pinId(p))}
              {@const src = iconOf(p)}
              {#if src}<img src={src} alt="" draggable="false" />{:else}<i class="ph">{pinName(p)[0]?.toUpperCase()}</i>{/if}
            {/each}
          </span>
          <span class="nm ellipsis">{f.name || 'Folder'}</span>
        </button>
      {/each}
    </div>
  </div>
{:else if folder}
  <div class="folderp" class:dropping>
    {#if picked}<button class="back" onclick={() => (picked = null)}><ChevronLeft size={14} /> All folders</button>{/if}
    <input class="name" value={folder.name} placeholder="Folder name (optional)" maxlength="32" aria-label="Folder name"
           onchange={(e) => save({ ...folder, name: e.currentTarget.value.trim() })}
           onkeydown={(e) => { if (e.key === 'Enter') e.currentTarget.blur(); }} />
    {#if picking}
      <AppPicker have={folder.apps} add={addOne} done={() => (picking = false)} />
    {:else}
    <div class="grid" bind:this={grid}>
      {#each shown as p, i (pinId(p))}
        {@const src = iconOf(p)}
        {@const on = wins.of(p).length > 0}
        <button class="app" class:lifted={dragKey === pinId(p)} class:leaving={dragKey === pinId(p) && outside}
                title={dragKey === pinId(p) && outside ? 'Release to take it out of the folder' : p.path}
                onpointerdown={(e) => down(e, p)} onclick={() => launch(p)} oncontextmenu={(e) => menu(e, p, i)}>
          {#if src}<img src={src} alt="" width="36" height="36" draggable="false" />{:else}<i class="ph">{pinName(p)[0]?.toUpperCase()}</i>{/if}
          <span class="nm ellipsis">{pinName(p)}</span>
          {#if on}<span class="run"></span>{/if}
        </button>
      {/each}
      <button class="app add" onclick={() => (picking = true)} title="Add apps to this folder"><span class="plus"><Plus size={18} /></span><span class="nm">Add app</span></button>
    </div>
    <p class="hint dim">{outside ? 'Release to take it out of the folder' : folder.apps.length ? 'Drag to reorder · drag out to remove · drop programs here to add' : 'Drop programs or shortcuts here, or use Add app'}</p>
    {/if}
  </div>
{:else}
  <p class="dim hint">This folder was deleted.</p>
{/if}

<style>
  .folderp { display: flex; flex-direction: column; gap: 10px; border-radius: var(--r-outer); transition: box-shadow 0.2s var(--ease); }
  .folderp.dropping { box-shadow: inset 0 0 0 2px var(--accent); }
  .name {
    height: 36px; padding: 0 10px; border-radius: var(--r-inner); border: 1px solid transparent; background: transparent;
    color: var(--fg); font: inherit; font-size: 16px; font-weight: 600; text-align: center;
    transition: background 0.2s var(--ease), border-color 0.2s var(--ease);
  }
  .name:hover { background: var(--pill); }
  .name:focus { outline: none; background: var(--bg-2, var(--pill)); border-color: var(--accent); }
  .name::placeholder { color: var(--fg-dim); font-weight: 500; }
  .grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px; }
  .app {
    position: relative; display: flex; flex-direction: column; align-items: center; gap: 4px; min-width: 0; padding: 8px 4px 6px;
    border-radius: var(--r-inner); touch-action: none;
    transition: background 0.2s var(--ease), transform 0.22s var(--spring), opacity 0.2s var(--ease);
    animation: pop 0.22s var(--ease) backwards;
  }
  .app:nth-child(2) { animation-delay: 30ms; } .app:nth-child(3) { animation-delay: 60ms; } .app:nth-child(4) { animation-delay: 90ms; }
  @keyframes pop { from { opacity: 0; transform: scale(0.9); } }
  .app:hover { background: var(--pill-hover); }
  .app:active { transform: scale(0.94); }
  .app:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .app.lifted { transform: scale(1.08); background: var(--pill-hover); box-shadow: 0 6px 18px rgba(0, 0, 0, 0.25); z-index: 1; }
  .app.leaving { opacity: 0.45; transform: scale(0.9); }
  .app img { width: 36px; height: 36px; object-fit: contain; }
  .ph { display: grid; place-items: center; width: 36px; height: 36px; border-radius: calc(10px * var(--rs)); font-style: normal; font-weight: 600;
        background: var(--accent); color: var(--on-accent); }
  .nm { max-width: 100%; font-size: 11px; text-align: center; }
  .run { position: absolute; top: 6px; right: 12px; width: 6px; height: 6px; border-radius: 50%; background: var(--accent); }
  .add { color: var(--fg-dim); }
  .plus { display: grid; place-items: center; width: 36px; height: 36px; border-radius: calc(10px * var(--rs)); border: 1.5px dashed var(--line); }
  .add:hover .plus { border-color: var(--accent); color: var(--accent); }
  .hint { margin: 0; font-size: 11px; text-align: center; }
  .title { margin: 0; font-size: 15px; font-weight: 600; text-align: center; }
  .back { display: flex; align-items: center; gap: 2px; align-self: flex-start; padding: 2px 8px 2px 4px; border-radius: var(--r-inner); font-size: 12px; color: var(--fg-dim); }
  .back:hover { background: var(--pill-hover); color: var(--fg); }
  /* A folder in the All folders list: its first four apps on a phone-style square. */
  .mini {
    display: grid; grid-template-columns: repeat(2, 16px); grid-auto-rows: 16px; gap: 2px; padding: 4px; width: 42px; height: 42px; box-sizing: border-box;
    border-radius: calc(11px * var(--rs)); background: var(--bg-2, var(--pill)); border: 1px solid var(--line-soft);
  }
  .mini img, .mini .ph { width: 16px; height: 16px; font-size: 9px; border-radius: 4px; object-fit: contain; }
  @media (prefers-reduced-motion: reduce) { .app { animation: none; transition: none; } }
</style>
