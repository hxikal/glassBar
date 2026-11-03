<script lang="ts">
  // App Folders on the dock: each folder is one tile showing its first four app icons in a
  // 2 x 2 grid, like a folder on a phone, with a dot when any app inside is running. A click
  // opens the folder (hovering too, when that option is on). Dropping programs or shortcuts
  // from Explorer onto a folder adds them to it. Only the first 2-4 folders show (maxShown);
  // a "+n" chip, or a click on the widget around the folders, opens all of them.
  import { FolderOpen, FolderPlus } from '@lucide/svelte';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { ipc } from '../../lib/ipc';
  import { updateWidgetSettings } from '../../lib/config.svelte';
  import { hoverPanel } from '../../lib/hoverPanel';
  import { showMenu } from '../../lib/menu';
  import { hits } from '../shelf/shelf';
  import type { PillProps } from '../types';
  import type { FoldersSettings } from './index';
  import { ADD_APP, ALL, addApps, folderKey, isApp, newFolder, pinName, replace, shownCount, visibleFolders, type Folder } from './folders';
  import type { Pin } from '../apps/index';
  import { iconOf, liveWindows } from './appIcons.svelte';

  let { instance, settings, vertical }: PillProps<FoldersSettings> = $props();
  const folders = $derived(settings.folders ?? []);
  const vis = $derived(visibleFolders(folders, settings.maxShown));
  const save = (next: Folder[]) => updateWidgetSettings(instance, { folders: next });
  const wins = liveWindows();
  const running = (f: Folder) => f.apps.some((p) => wins.of(p).length > 0);

  const W = 360, H = 300;
  const hp = hoverPanel();
  let tiles = $state<Record<string, HTMLElement>>({});
  function open(e: MouseEvent, f: Folder) {
    e.stopPropagation();
    hp.cancel();
    ipc.popupOpen(folderKey(false, instance, f.id), (e.currentTarget as HTMLElement).getBoundingClientRect(), W, H);
  }
  function openAll(e: MouseEvent) {
    e.stopPropagation();
    if (!folders.length || !wrap) return;
    hp.cancel();
    ipc.popupOpen(folderKey(false, instance, ALL), wrap.getBoundingClientRect(), W, H);
  }
  const enter = (e: PointerEvent, f: Folder) => { if (settings.openOnHover) hp.enter(e, folderKey(true, instance, f.id), W, H); };

  async function menu(e: MouseEvent, f?: Folder) {
    e.preventDefault();
    e.stopPropagation();
    await showMenu([
      ...(f ? [{ text: 'Open', action: () => ipc.popupOpen(folderKey(false, instance, f.id), tiles[f.id].getBoundingClientRect(), W, H) }] : []),
      ...(folders.length > 1 ? [{ text: 'All folders', action: () => wrap && ipc.popupOpen(folderKey(false, instance, ALL), wrap.getBoundingClientRect(), W, H) }] : []),
      { text: 'New folder', action: () => save([...folders, newFolder()]) },
      { text: 'Folders on the dock', items: [2, 3, 4].map((n) => ({ text: `${n}`, checked: shownCount(settings.maxShown) === n,
        action: () => updateWidgetSettings(instance, { maxShown: n }) })) },
      { text: 'Show folder names', checked: settings.showNames, action: () => updateWidgetSettings(instance, { showNames: !settings.showNames }) },
      { text: 'Open folders on hover', checked: settings.openOnHover, action: () => updateWidgetSettings(instance, { openOnHover: !settings.openOnHover }) },
      // Deletes at once: the folder's shortcuts go, the apps themselves are untouched.
      ...(f ? ['-' as const, { text: 'Delete folder', action: () => save(folders.filter((x) => x.id !== f.id)) }] : []),
    ]);
  }

  // An icon dragged from the App Launcher and released on a folder (see ADD_APP).
  let wrap = $state<HTMLElement>();
  let landed = $state<string | null>(null);
  let landedTimer: ReturnType<typeof setTimeout>;
  function put(f: Folder, apps: Pin[]) {
    save(replace(folders, addApps(f, apps)));
    landed = f.id; // a little pop, so you see where it went
    clearTimeout(landedTimer);
    landedTimer = setTimeout(() => (landed = null), 500);
  }
  $effect(() => {
    const node = wrap;
    if (!node) return;
    const onAdd = (e: Event) => {
      const id = (e.target as HTMLElement).closest<HTMLElement>('[data-folder-drop]')?.dataset.folderDrop;
      const f = folders.find((x) => x.id === id);
      if (f) put(f, [(e as CustomEvent<Pin>).detail]);
    };
    node.addEventListener(ADD_APP, onAdd);
    return () => { node.removeEventListener(ADD_APP, onAdd); clearTimeout(landedTimer); };
  });

  // Programs or shortcuts dragged in from Explorer: into the folder under the pointer.
  let over = $state<string | null>(null);
  $effect(() => {
    const off = getCurrentWebviewWindow().onDragDropEvent((e) => {
      const d = e.payload;
      if (d.type === 'leave') { over = null; return; }
      const at = folders.find((f) => tiles[f.id] && hits(tiles[f.id].getBoundingClientRect(), d.position.x, d.position.y, devicePixelRatio));
      if (d.type === 'enter' || d.type === 'over') { over = at?.id ?? null; return; }
      over = null;
      const apps = d.paths.filter(isApp).map((path) => ({ path }));
      if (at && apps.length) put(at, apps);
    });
    return () => { off.then((f) => f()).catch(() => {}); };
  });
</script>

<!-- The click around the folders opens all of them; keyboard users have +n and the menu. -->
<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div class={vertical ? 'side-stack' : 'pill folders'} bind:this={wrap} oncontextmenu={(e) => menu(e)} onclick={openAll}>
  {#each vis.shown as f (f.id)}
    {@const first = f.apps.slice(0, 4)}
    <button class="folder" class:side={vertical} class:over={over === f.id} class:landed={landed === f.id} bind:this={tiles[f.id]} data-folder-drop={f.id}
            title={`${f.name ? `${f.name} · ` : ''}${f.apps.length ? f.apps.map(pinName).join(', ') : 'Empty — click to add apps, or drag one here'}`}
            onclick={(e) => open(e, f)} oncontextmenu={(e) => menu(e, f)}
            onpointerenter={(e) => enter(e, f)} onpointerleave={hp.leave}>
      <span class="grid" class:empty={!first.length}>
        {#if first.length}
          {#each first as p (p.path)}
            {@const src = iconOf(p)}
            {#if src}<img src={src} alt="" draggable="false" />{:else}<i class="ph">{pinName(p)[0]?.toUpperCase()}</i>{/if}
          {/each}
        {:else}
          <FolderOpen size={16} />
        {/if}
      </span>
      {#if running(f)}<span class="run" aria-label="An app in this folder is running"></span>{/if}
      {#if settings.showNames && f.name && !vertical}<span class="nm ellipsis">{f.name}</span>{/if}
    </button>
  {/each}
  {#if vis.more}
    <button class="folder more" class:side={vertical} title="All folders" onclick={openAll}><span class="grid empty">+{vis.more}</span></button>
  {/if}
  {#if !folders.length}
    <button class="folder" class:side={vertical} title="New folder" onclick={(e) => { e.stopPropagation(); save([newFolder()]); }}>
      <FolderPlus size={18} />
    </button>
  {/if}
</div>

<style>
  .folders { gap: 2px; padding: 0 4px; }
  .folder {
    position: relative; display: flex; align-items: center; gap: 6px; padding: 3px; border-radius: calc(10px * var(--rs));
    transition: background 0.2s var(--ease), transform 0.25s var(--spring), box-shadow 0.2s var(--ease);
  }
  .folder:hover { background: var(--pill-hover); }
  .folder:active { transform: scale(0.92); }
  .folder:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
  .folder.side { justify-content: center; padding: 6px 0; }
  /* A file dragged over this folder from Explorer. */
  .folder.over { box-shadow: inset 0 0 0 2px var(--accent); transform: scale(1.06); }
  /* An App Launcher icon is being dragged: every folder shows it can take it; the one under
     the pointer grows. */
  :global(html[data-app-drag]) .folder { box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--accent) 55%, transparent); }
  :global(html[data-app-drag]) .folder:hover { box-shadow: inset 0 0 0 2px var(--accent); transform: scale(1.1); background: var(--pill-hover); }
  .folder.landed .grid { animation: landed 0.45s var(--spring, ease); }
  @keyframes landed { 40% { transform: scale(1.18); } }
  @media (prefers-reduced-motion: reduce) { .folder.landed .grid { animation: none; } }
  /* The 2 x 2 preview, on a soft rounded square like a phone folder. */
  .grid {
    display: grid; grid-template-columns: repeat(2, 12px); grid-auto-rows: 12px; gap: 2px; padding: 3px;
    width: 32px; height: 32px; box-sizing: border-box; border-radius: calc(9px * var(--rs));
    background: var(--bg-2, var(--pill)); border: 1px solid var(--line-soft);
  }
  .grid.empty { display: grid; place-items: center; grid-template-columns: 1fr; color: var(--fg-dim); }
  .grid img { width: 12px; height: 12px; object-fit: contain; }
  .ph { display: grid; place-items: center; width: 12px; height: 12px; border-radius: 3px; font-size: 8px; font-style: normal; font-weight: 600;
        background: var(--accent); color: var(--on-accent); }
  .run { position: absolute; left: 50%; bottom: 0; width: 5px; height: 3px; translate: -50% 0; border-radius: 2px; background: var(--accent); }
  .more .grid { font-size: 12px; font-weight: 600; color: var(--fg); }
  .nm { max-width: 90px; font-size: 11.5px; font-weight: 500; }
</style>
