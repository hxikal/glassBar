<script lang="ts">
  // Settings › Storage: what GlassBar keeps on disk, by owner, and cleanup that says exactly
  // what it removes and asks first (storage.rs re-checks everything before deleting).
  import { onMount } from 'svelte';
  import { FolderOpen, RefreshCw, Trash2, TriangleAlert } from '@lucide/svelte';
  import { ipc, type StorageAction, type StorageGroup, type StorageReport, type StorageRow } from '../lib/ipc';
  import { GROUP_TITLE, fmtDay, fmtSize, sections, shares } from './storagePage';

  let report = $state<StorageReport | null>(null);
  let error = $state('');
  let busy = $state(false);
  let done = $state('');
  let pending = $state<{ row: string; a: StorageAction } | null>(null);

  async function scan() {
    busy = true; error = '';
    try { report = await ipc.storageScan(); } catch (e) { error = String(e); } finally { busy = false; }
  }
  onMount(scan);

  async function run() {
    const p = pending!;
    pending = null; busy = true; done = ''; error = '';
    try {
      const r = await ipc.storageClean(p.a.id);
      report = r.report;
      done = `${p.row}: freed ${fmtSize(r.freed)}.` + (r.backup ? ` The database was copied to ${r.backup.split('\\').pop()} first.` : '');
    } catch (e) { error = `${p.row}: ${e}`; } finally { busy = false; }
  }

  const groups = $derived(report ? sections(report.rows) : []);
  const bar = $derived(report ? shares(groups, report.total) : []);
  const COLOR: Record<StorageGroup, string> = {
    leftover: '#ff9f0a', widget: 'var(--accent)', app: 'color-mix(in srgb, var(--accent) 45%, var(--fg-dim))', other: 'var(--fg-dim)',
  };
  const LEGEND: Record<StorageGroup, string> = { leftover: 'Leftovers', widget: 'Widgets', app: 'App data', other: 'Other' };
  const web = $derived(report?.webview);
  const webAction = $derived<StorageAction | null>(web && web[2] > 0
    ? { id: 'webview', label: 'Clear cache', bytes: web[2], destructive: false,
        detail: `Asks WebView2 to clear its disk cache: up to ${fmtSize(web[2])} of copies of GlassBar's pages and scripts it keeps to start faster. WebView2 keeps what the open windows are using, so it usually frees less, and rebuilds the rest when needed.` }
    : null);
  const reveal = (name: string) => report && ipc.downloadsReveal(`${report.dir}\\${name}`).catch(() => {});
  const many = (r: StorageRow) => r.group === 'leftover' && r.items.length > 1;
</script>

{#if !report}
  <p class="hint">{error || 'Measuring…'}</p>
{:else}
  <div class="top">
    <b>{fmtSize(report.total)}</b>
    <span class="dim path" title={report.dir}>in {report.dir}</span>
    <button class="act" onclick={scan} disabled={busy}><RefreshCw size={13} /> Rescan</button>
  </div>
  <div class="split" role="img" aria-label="How the space is split">
    {#each bar as s (s.group)}<i style="width:{s.pct}%; background:{COLOR[s.group]}"></i>{/each}
  </div>
  <div class="legend">
    {#each groups as g (g.group)}<span><i style="background:{COLOR[g.group]}"></i>{LEGEND[g.group]} {fmtSize(g.bytes)}</span>{/each}
  </div>
  {#if done}<p class="msg ok">{done}</p>{/if}
  {#if error}<p class="msg bad">{error}</p>{/if}

  {#each groups as g (g.group)}
    <h2 class="gh"><span>{GROUP_TITLE[g.group]}</span><span>{fmtSize(g.bytes)}</span></h2>
    <div class="group">
      {#each g.rows as r (r.name)}
        <div class="srow">
          <div class="what">
            <b>{r.name}</b>
            <span class="dim">{r.note}{#if r.group === 'leftover' && !many(r) && r.items[0]}{' · '}{fmtDay(r.items[0].modified)}{/if}</span>
            {#if many(r)}
              <details>
                <summary class="dim">Show {r.items.length}</summary>
                {#each r.items as i, n (i.name)}
                  <div class="item"><button class="link" onclick={() => reveal(i.name)} title="Show in Explorer">{i.name}</button>
                    <span class="dim">{fmtDay(i.modified)}{#if n === 0} · newest, kept{/if}</span><span class="num">{fmtSize(i.bytes)}</span></div>
                {/each}
              </details>
            {/if}
          </div>
          <span class="num">{fmtSize(r.bytes)}</span>
          <div class="acts">
            {#if !many(r) && r.items.find((i) => i.dir)}
              <button class="act" onclick={() => reveal(r.items.find((i) => i.dir)!.name)} title="Show in Explorer" aria-label="Show {r.name} in Explorer"><FolderOpen size={13} /></button>
            {/if}
            {#each r.actions as a (a.id)}
              <button class="act" class:danger={a.destructive} disabled={busy} onclick={() => (pending = { row: r.name, a })}>
                {#if a.destructive}<Trash2 size={13} />{/if}{a.label}
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/each}

  {#if web}
    <h2 class="gh"><span>Elsewhere · WebView2, the browser engine GlassBar draws with</span><span>{fmtSize(web[1])}</span></h2>
    <div class="group">
      <div class="srow">
        <div class="what"><b>WebView2 data</b><span class="dim">{fmtSize(web[2])} of it is cache · {web[0]}</span></div>
        <span class="num">{fmtSize(web[1])}</span>
        <div class="acts">{#if webAction}<button class="act" disabled={busy} onclick={() => (pending = { row: 'WebView2', a: webAction })}>Clear cache</button>{/if}</div>
      </div>
    </div>
  {/if}
  <p class="hint">Sizes add up the way Explorer's Properties does. Cleanups delete only inside GlassBar's own folders, never files a widget merely points to (Shelf, Downloads, your azan sound). Before a cleanup touches the database it is copied to a dated backup-storage folder, and compacted afterwards so the space is really freed.</p>
{/if}

{#if pending}
  <div class="veil" role="presentation" onclick={(e) => e.target === e.currentTarget && (pending = null)}>
    <div class="dlg" role="alertdialog" aria-modal="true" aria-labelledby="dlg-t" aria-describedby="dlg-d">
      <b id="dlg-t">{#if pending.a.destructive}<TriangleAlert size={15} class="warn" />{/if} {pending.row}: {pending.a.label.toLowerCase()}?</b>
      <p id="dlg-d">{pending.a.detail}
        {#if pending.a.destructive} This can't be undone from GlassBar.{/if}</p>
      <div class="dbtn">
        <button class="act" onclick={() => (pending = null)}>Cancel</button>
        <!-- svelte-ignore a11y_autofocus -->
        <button class="btn" class:danger-fill={pending.a.destructive} onclick={run} autofocus>
          {pending.a.destructive ? `Delete ${fmtSize(pending.a.bytes)}` : pending.a.label}
        </button>
      </div>
    </div>
  </div>
{/if}

<svelte:window onkeydown={(e) => e.key === 'Escape' && (pending = null)} />

<style>
  .top { display: flex; align-items: baseline; gap: 10px; margin: 6px 0 0; }
  .top b { font-size: 24px; font-weight: 600; font-variant-numeric: tabular-nums; white-space: nowrap; }
  .path { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .top .act { align-self: center; flex: none; }
  .act { display: inline-flex; align-items: center; gap: 5px; padding: 5px 11px; font-size: 12px; border-radius: calc(9px * var(--rs));
         background: var(--pill); color: var(--fg); white-space: nowrap; }
  .act:hover:not(:disabled) { background: var(--pill-hover); }
  .act:disabled { opacity: 0.5; }
  .dim { color: var(--fg-dim); font-size: 12px; }
  .split { display: flex; height: 12px; border-radius: 6px; overflow: hidden; margin: 10px 0 6px; background: var(--field-bg); }
  .split i { display: block; height: 100%; }
  .legend { display: flex; flex-wrap: wrap; gap: 4px 14px; font-size: 12px; color: var(--fg-dim); }
  .legend i { display: inline-block; width: 10px; height: 10px; border-radius: 3px; margin-right: 6px; vertical-align: -1px; }
  .gh { display: flex; justify-content: space-between; margin-right: 4px !important; }
  .srow { display: grid; grid-template-columns: minmax(0, 1fr) auto auto; gap: 14px; align-items: center; padding: 10px 0; }
  .srow + .srow { border-top: 1px solid var(--line-soft); }
  .what { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .what b { font-weight: 500; }
  .num { font-variant-numeric: tabular-nums; text-align: right; white-space: nowrap; font-size: 13px; }
  .acts { display: flex; gap: 6px; justify-content: flex-end; flex-wrap: wrap; }
  .danger { color: #ff453a; }
  :global(html[data-theme='light']) .danger { color: #c4271d; }
  .danger-fill { background: #d93025; color: #fff; }
  details summary { cursor: pointer; width: fit-content; }
  .item { display: grid; grid-template-columns: minmax(0, 1fr) auto auto; gap: 12px; padding: 3px 0 3px 12px; font-size: 12px; }
  .link { text-align: left; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--fg); }
  .link:hover { text-decoration: underline; }
  .msg { margin: 10px 0 0; padding: 7px 10px; border-radius: calc(8px * var(--rs)); font-size: 12.5px; }
  .ok { background: color-mix(in srgb, #30d158 18%, transparent); }
  .bad { background: color-mix(in srgb, #ff453a 18%, transparent); }
  .veil { max-width: none !important; position: fixed; inset: 0; display: grid; place-items: center; background: rgb(0 0 0 / 0.45); z-index: 10; }
  .dlg { width: min(420px, calc(100vw - 32px)); padding: 16px 18px; border-radius: calc(14px * var(--rs)); background: var(--panel-solid, var(--panel)); border: 1px solid var(--line-soft); box-shadow: 0 12px 40px rgb(0 0 0 / 0.35); }
  .dlg b { display: flex; align-items: center; gap: 6px; font-size: 15px; font-weight: 600; }
  .dlg p { margin: 8px 0 0; font-size: 13px; line-height: 1.5; color: var(--fg-dim); }
  .dlg :global(.warn) { color: #ff9f0a; flex: none; }
  .dbtn { display: flex; justify-content: flex-end; gap: 8px; margin-top: 14px; }
</style>
