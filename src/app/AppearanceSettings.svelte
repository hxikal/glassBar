<script lang="ts">
  // Settings › Appearance: theme, design style, materials, colours, advanced, presets.
  // Every change shows on the real dock: sliders live while dragging (previewTheme), saved on release.
  import { RotateCcw, Trash2 } from '@lucide/svelte';
  import { profile, previewTheme, store, update } from '../lib/config.svelte';
  import { events, ipc, type AppearanceSupport, type Theme } from '../lib/ipc';
  import { ACRYLIC_BASE, applySnapshot, changed, FLOOR, luminance, presetDefaults, resolve, snapshot, STYLES, THEMES, windowRadius, type Style, type ThemePreset } from '../lib/appearance';
  import { settingsOf, widgets } from '../widgets/registry';
  import type { Edge } from '../lib/geometry';
  import { backdrop } from '../lib/theme';

  type Key = keyof ReturnType<typeof presetDefaults>;
  const saved = $derived(store.config!.theme);
  let draft = $state<Theme | null>(null);
  const t = $derived(draft ?? saved);

  let sup = $state<AppearanceSupport | null>(null);
  $effect(() => { void saved; ipc.appearanceSupport().then((s) => (sup = s)).catch(() => {}); });
  $effect(() => { const off = events.materialChanged((m) => { if (sup) sup = { ...sup, applied: m }; }); return () => { off.then((f) => f()); }; });

  const prefersDark = matchMedia('(prefers-color-scheme: dark)').matches;
  let behind = $state(backdrop.behind);
  $effect(() => { const off = events.behindChanged((b) => (behind = b)); return () => { off.then((f) => f()); }; });
  const r = $derived(resolve(t, { transparency: sup?.transparency ?? true, prefersDark, behind }));

  const save = (f: (t: Theme) => void) => { draft = null; return update((c) => f(c.theme)); };
  const setKey = <K extends keyof Theme>(k: K, v: Theme[K]) => save((th) => { th[k] = v; });
  const live = <K extends keyof Theme>(k: K, v: Theme[K]) => { draft = { ...t, [k]: v }; previewTheme(draft); };
  const revert = (k: Key) => (k === 'bg' ? setBg(presetDefaults(t).bg) : setKey(k, presetDefaults(t)[k] as never));
  // Windows draws the material light or dark: keep it matching the text a custom colour gets.
  const setBg = (bg: string | null) => save((th) => { th.bg = bg; if (bg) th.mode = luminance(bg) < 0.4 ? 'dark' : 'light'; });

  const pickTheme = (p: ThemePreset) => save((th) => {
    Object.assign(th, THEMES[p].set, { preset: p });
    const s = THEMES[p].style;
    if (s) Object.assign(th, STYLES[s].set, { style: s });
  });
  const pickStyle = (s: Style) => save((th) => { Object.assign(th, STYLES[s].set, { style: s }); });
  const modified = $derived((Object.keys(presetDefaults(t)) as Key[]).filter((k) => changed(t, k)));

  // Two-step confirm for resets (click again within 4 s).
  let arm = $state<string | null>(null);
  let armTimer: ReturnType<typeof setTimeout>;
  function confirmed(id: string) {
    if (arm === id) { arm = null; return true; }
    arm = id; clearTimeout(armTimer); armTimer = setTimeout(() => (arm = null), 4000);
    return false;
  }
  const resetPreset = () => confirmed('preset') && save((th) => { Object.assign(th, presetDefaults(th)); });
  const resetAll = () => confirmed('all') && save((th) => {
    Object.assign(th, THEMES.system.set, STYLES.glassbar.set, { preset: 'system', style: 'glassbar', accent: '#0A84FF' });
  });

  let presetName = $state('');
  function savePreset() {
    const name = presetName.trim();
    if (!name) return;
    save((th) => { th.saved = { ...th.saved, [name]: snapshot(th) }; });
    presetName = '';
  }

  // Preview: a fake wallpaper with the dock (real widget components) and a panel on it.
  let edge = $state<Edge>(profile().bar.edge);
  const vertical = $derived(edge === 'left' || edge === 'right');
  const items = $derived(profile().widgets.filter((w) => w.enabled !== false && widgets[w.id]).slice(0, 6));
  const pct = (v: number) => `${Math.round(v * 100)}%`;
  const MATERIALS = [['blur', 'Blur'], ['acrylic', 'Acrylic'], ['mica', 'Mica']] as const;
  const materialName = (m: string) => MATERIALS.find(([k]) => k === m)?.[1] ?? 'none';
</script>

{#snippet revertBtn(k: Key)}
  {#if changed(t, k)}<button class="rev" title="Back to the preset's value" aria-label="Revert" onclick={() => revert(k)}><RotateCcw size={13} /></button>{/if}
{/snippet}

{#snippet slider(k: 'opacity' | 'panelOpacity' | 'cardOpacity' | 'shadow' | 'border' | 'iconScale' | 'spacing' | 'radius' | 'blur', label: string, min: number, max: number, step: number, fmt: (v: number) => string, disabled = false)}
  <div class="field" class:off={disabled}><span>{label} <b class="val">{fmt(t[k])}</b></span>
    <span class="ctl">
      {@render revertBtn(k)}
      <input type="range" {min} {max} {step} value={t[k]} {disabled} aria-label={label}
             oninput={(e) => live(k, +e.currentTarget.value)} onchange={(e) => setKey(k, +e.currentTarget.value)} />
    </span>
  </div>
{/snippet}

<!-- Preview -->
<div class="preview">
  <div class="wall {edge}">
    <div class="pvdock dock" class:vertical class:blur={r.blurred} class:mica={r.blurred && sup?.applied === 'mica'}
         style:--tint-alpha={r.dock} style:--base={r.blurred && sup?.applied === 'acrylic' ? ACRYLIC_BASE[r.dark ? 'dark' : 'light'] : 0} style:--r-window="{windowRadius(t.radius, r.blurred)}px">
      {#each items as w (w.instance)}
        {@const m = widgets[w.id]}
        <div class="slot"><div class="inert" inert><m.Pill instance={w.instance} size={vertical ? 'small' : w.size} settings={settingsOf(m, w)} {vertical} /></div></div>
      {/each}
    </div>
    <div class="pvpanel" class:blur={r.blurred} style:--tint-alpha={r.panel} style:--base={r.blurred && sup?.applied === 'acrylic' ? ACRYLIC_BASE[r.dark ? 'dark' : 'light'] : 0}>
      <b>Panel</b>
      <span class="dim">Flyouts, Quick Settings, menus</span>
      <div class="row"><span class="tile on">Wi-Fi</span><span class="tile">Bluetooth</span></div>
    </div>
  </div>
  <div class="pvbar">
    <span class="dim">Preview · the dock itself changes too</span>
    <span class="seg">{#each ['top', 'bottom', 'left', 'right'] as const as e (e)}<button class:sel={edge === e} onclick={() => (edge = e)}>{e}</button>{/each}</span>
  </div>
</div>

<h2>Theme</h2>
<div class="cards">
  {#each Object.entries(THEMES) as [k, th] (k)}
    <button class="choice" class:sel={t.preset === k} onclick={() => pickTheme(k as ThemePreset)}>
      <span class="sw theme-{k}"></span><b>{th.label}</b><span class="hint">{th.hint}</span>
    </button>
  {/each}
</div>

<h2>Design style</h2>
<div class="cards">
  {#each Object.entries(STYLES) as [k, st] (k)}
    <button class="choice" class:sel={t.style === k} onclick={() => pickStyle(k as Style)}>
      <span class="shape" style:border-radius="{st.set.radius * 0.8}px"></span><b>{st.label}</b><span class="hint">{st.hint}</span>
    </button>
  {/each}
</div>
<p class="hint">Styles are visual inspirations only: GlassBar keeps working the same way and Windows itself doesn't change.</p>

<h2>Materials</h2>
<div class="group">
  <div class="field"><span>Background blur</span>
    <span class="ctl">{@render revertBtn('nativeBlur')}<input type="checkbox" class="switch" aria-label="Background blur" checked={t.nativeBlur} onchange={(e) => setKey('nativeBlur', e.currentTarget.checked)} /></span>
  </div>
  {#if sup && !sup.transparency}
    <p class="hint warn">Transparency effects are off in Windows (Settings › Personalization › Colors), so Windows can't blur anything. GlassBar draws a solid background until you turn them on.</p>
  {/if}
  <div class="field" class:off={!t.nativeBlur}><span>Material</span>
    <span class="ctl">{@render revertBtn('material')}
      <span class="seg">{#each MATERIALS as [k, label] (k)}
        <button class:sel={t.material === k} disabled={!t.nativeBlur || (sup ? !sup[k] : false)} title={sup && !sup[k] ? 'Needs Windows 11' : ''}
                onclick={() => setKey('material', k)}>{label}{#if sup && !sup[k]} <small>(Win 11)</small>{/if}</button>
      {/each}</span>
    </span>
  </div>
  {#if t.nativeBlur && sup?.transparency}
    {#if sup.applied === 'none'}
      <p class="hint warn">Windows couldn't draw a material here, so the dock has no blur: GlassBar uses a solid tint instead.</p>
    {:else if sup.applied !== t.material}
      <p class="hint warn">{materialName(t.material)} isn't available on this Windows version: it's drawing {materialName(sup.applied)} instead.</p>
    {:else}
      <p class="hint">
        {#if t.material === 'acrylic'}Windows' frosted glass. Windows adds its own milky layer (about {Math.round(ACRYLIC_BASE[r.dark ? 'dark' : 'light'] * 100)}% here) under your opacity, so it never gets fully clear.
        {:else if t.material === 'mica'}Tinted by your wallpaper, barely see-through. Windows shows Mica plainly while GlassBar isn't the active window, which is most of the time.
        {:else}Clear blur: what's behind shows through, and the opacity below decides how much.{/if}
        Windows sets the blur strength itself; apps can't change it.
      </p>
    {/if}
  {:else if !t.nativeBlur}
    <p class="hint">Off: no blur, only the tint below (a page can't blur the desktop behind its window).</p>
  {/if}
  {@render slider('opacity', 'Dock opacity', 0, 1, 0.01, pct)}
  {#if r.raised.dock && sup?.transparency !== false}<p class="hint">Shown at {pct(r.dock)}: {r.wall >= r.dock ? `your wallpaper behind the dock is ${r.dark ? 'bright' : 'dark'}, so ${r.dark ? 'white' : 'dark'} text` : r.blurred ? 'high-contrast text' : 'without blur, text'} needs a more solid background to stay readable.</p>{/if}
  {@render slider('panelOpacity', 'Panel opacity', 0, 1, 0.01, pct)}
  {#if r.raised.panel && sup?.transparency !== false}<p class="hint">Shown at {pct(r.panel)} so panel text stays readable.</p>{/if}
  {@render slider('cardOpacity', 'Widget card opacity', 0, 1, 0.01, pct)}
  {#if r.raised.card && sup?.transparency !== false}<p class="hint">Shown at {pct(r.card)}: below {pct(FLOOR.card)} the widgets stop looking clickable.</p>{/if}
  <p class="hint">0% = only the Windows material. Panels covers flyouts, Quick Settings, Wi-Fi and Bluetooth lists and other expanded widgets.</p>
</div>

<h2>Colours</h2>
<div class="group">
  <label class="field"><span>Accent colour</span>
    <input type="color" value={t.accent} oninput={(e) => live('accent', e.currentTarget.value)} onchange={(e) => setKey('accent', e.currentTarget.value)} />
  </label>
  <div class="field" class:off={!!t.bg}><span>Light or dark</span>
    <span class="ctl">{@render revertBtn('mode')}
      <span class="seg">{#each [['auto', 'Follow Windows'], ['light', 'Light'], ['dark', 'Dark']] as const as [m, label] (m)}
        <button class:sel={t.mode === m} disabled={!!t.bg} onclick={() => setKey('mode', m)}>{label}</button>
      {/each}</span>
    </span>
  </div>
  <div class="field"><span>Custom background colour</span>
    <span class="ctl">{@render revertBtn('bg')}
      {#if t.bg}<input type="color" aria-label="Background colour" value={t.bg} oninput={(e) => live('bg', e.currentTarget.value)} onchange={(e) => setBg(e.currentTarget.value)} />{/if}
      <input type="checkbox" class="switch" aria-label="Custom background colour" checked={!!t.bg} onchange={(e) => setBg(e.currentTarget.checked ? (r.dark ? '#1e2433' : '#eef1f6') : null)} />
    </span>
  </div>
  {#if t.bg}<p class="hint">Text turns {r.dark ? 'light' : 'dark'} automatically to read on this colour.</p>{/if}
  <div class="field"><span>Text and icon contrast</span>
    <span class="ctl">{@render revertBtn('contrast')}
      <span class="seg"><button class:sel={t.contrast === 'normal'} onclick={() => setKey('contrast', 'normal')}>Normal</button>
      <button class:sel={t.contrast === 'high'} onclick={() => setKey('contrast', 'high')}>High</button></span>
    </span>
  </div>
</div>

<details class="adv">
  <summary><h2>Advanced</h2></summary>
  <div class="group">
    {@render slider('radius', 'Corner radius', 0, 24, 1, (v) => `${v}px`)}
    {#if r.blurred && t.radius > 8}<p class="hint">With a Windows material the dock window's own corners stay at 8 px (Windows rounds them); everything inside uses {t.radius} px.</p>{/if}
    {@render slider('shadow', 'Shadow', 0, 2, 0.05, pct)}
    {@render slider('border', 'Borders', 0, 2, 0.05, pct)}
    {@render slider('iconScale', 'Dock icon size', 0.8, 1.4, 0.05, pct)}
    {@render slider('spacing', 'Dock spacing', 0.6, 1.6, 0.05, pct)}
    <div class="field"><span>Animations</span>
      <span class="ctl">{@render revertBtn('motion')}
        <span class="seg">{#each [['full', 'Full'], ['reduced', 'Calm'], ['off', 'Off']] as const as [m, label] (m)}
          <button class:sel={t.motion === m} onclick={() => setKey('motion', m)}>{label}</button>
        {/each}</span>
      </span>
    </div>
    <p class="hint">Calm: no springy bounce. Off: nothing moves. Windows' "Animation effects" setting off also turns them off.</p>
    {@render slider('blur', 'Overlay blur', 0, 60, 1, (v) => `${v}px`)}
    <p class="hint">Blur behind overlays drawn inside GlassBar (e.g. the Snap hint over a screenshot). The blur behind the dock is the Windows material above.</p>
  </div>
</details>

<h2>Presets</h2>
<div class="group">
  <div class="field"><span>{THEMES[t.preset].label} + {STYLES[t.style].label}{#if modified.length} <small class="dim">· {modified.length} changed</small>{/if}</span>
    <span class="ctl">
      <button class="btn plain" disabled={!modified.length} onclick={resetPreset}>{arm === 'preset' ? 'Click again to reset' : 'Reset to preset'}</button>
      <button class="btn danger" onclick={resetAll}>{arm === 'all' ? 'Click again: reset everything' : 'Reset all appearance'}</button>
    </span>
  </div>
  {#each Object.keys(t.saved) as name (name)}
    <div class="wrow">
      <b>{name}</b>
      <button class="btn plain" onclick={() => save((th) => applySnapshot(th, th.saved[name]))}>Apply</button>
      <button class="icon-btn" aria-label="Delete {name}" onclick={() => save((th) => { const { [name]: _, ...rest } = th.saved; th.saved = rest; })}><Trash2 size={14} /></button>
    </div>
  {/each}
  <div class="wrow">
    <input class="grow" placeholder="Save the current look as…" bind:value={presetName} onkeydown={(e) => e.key === 'Enter' && savePreset()} />
    <button class="btn" disabled={!presetName.trim()} onclick={savePreset}>Save preset</button>
  </div>
</div>

<style>
  .preview { position: sticky; top: -18px; z-index: 2; margin: 0 0 6px; padding-top: 4px; background: var(--settings-bg); }
  /* A made-up wallpaper with sharp shapes, so blur and opacity are easy to judge. */
  .wall {
    position: relative; height: 210px; border-radius: 14px; overflow: hidden; display: flex; padding: 10px; gap: 10px;
    background:
      linear-gradient(115deg, transparent 0 38%, rgba(255, 255, 255, 0.85) 38% 41%, transparent 41%),
      radial-gradient(40% 70% at 18% 30%, #ff6b4a, transparent 70%),
      radial-gradient(45% 80% at 70% 80%, #2f7dff, transparent 70%),
      radial-gradient(30% 60% at 90% 10%, #ffd23f, transparent 70%),
      #3a2f5b;
  }
  .wall.left, .wall.right { height: 280px; }
  .wall.top { flex-direction: column; align-items: center; }
  .wall.bottom { flex-direction: column-reverse; align-items: center; }
  .wall.left { flex-direction: row; }
  .wall.right { flex-direction: row-reverse; }
  .pvdock {
    display: flex; gap: var(--gap); padding: calc(5px * var(--sp)); height: 48px; flex: none; max-width: 100%; overflow: hidden;
    background: linear-gradient(rgba(var(--tint), var(--tint-alpha)), rgba(var(--tint), var(--tint-alpha))), rgba(var(--tint), var(--base, 0)); border-radius: var(--r-window);
    border: 1px solid var(--line); box-shadow: var(--shadow); color: var(--fg);
  }
  .pvdock.vertical { flex-direction: column; height: auto; width: 72px; max-height: 100%; }
  .pvdock .slot .inert { display: flex; width: 100%; }
  /* Real blur here: of the wallpaper drawn above, inside this page (Windows does the desktop's). */
  .blur { backdrop-filter: blur(26px) saturate(160%); }
  /* Mica: Windows draws it flat while GlassBar isn't the active window, i.e. nearly always. */
  .pvdock.mica { backdrop-filter: none; background: rgba(var(--tint), max(0.92, var(--tint-alpha))); }
  .pvpanel {
    display: flex; flex-direction: column; gap: 4px; padding: 10px 12px; width: 190px; flex: none; align-self: center;
    background: linear-gradient(rgba(var(--tint), var(--tint-alpha)), rgba(var(--tint), var(--tint-alpha))), rgba(var(--tint), var(--base, 0)); border-radius: var(--r-outer); border: 1px solid var(--line);
    box-shadow: var(--shadow); color: var(--fg); font-size: 12px;
  }
  .wall.top .pvpanel, .wall.bottom .pvpanel { align-self: flex-end; }
  .pvpanel .row { display: flex; gap: 6px; }
  .pvpanel .tile { padding: 6px 10px; font-size: 11px; }
  .pvbar { display: flex; justify-content: space-between; align-items: center; padding: 6px 2px; font-size: 12px; }

  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 8px; }
  .choice { display: flex; flex-direction: column; align-items: flex-start; gap: 4px; padding: 10px 12px; text-align: left;
    border-radius: calc(12px * var(--rs)); background: var(--panel); border: 1px solid var(--line-soft); }
  .choice.sel { outline: 2px solid var(--accent); outline-offset: -1px; }
  .choice .hint { margin: 0; }
  .sw, .shape { width: 100%; height: 28px; border-radius: 8px; border: 1px solid var(--line-soft); margin-bottom: 4px; }
  .theme-light { background: #f6f6fa; }
  .theme-dark { background: #1c1c20; }
  .theme-glass { background: linear-gradient(135deg, rgba(255, 255, 255, 0.5), rgba(255, 255, 255, 0.08)), linear-gradient(90deg, #ff6b4a, #2f7dff); }
  .theme-system { background: linear-gradient(90deg, #f6f6fa 50%, #1c1c20 50%); }
  .theme-liquid { background: radial-gradient(120% 70% at 30% 0%, rgba(255, 255, 255, 0.7), transparent 60%), linear-gradient(135deg, #3a7bfd, #8f5bff 60%, #ff7ac6); }
  .theme-custom { background: conic-gradient(#ff6b4a, #ffd23f, #3ad29f, #2f7dff, #b04aff, #ff6b4a); }
  .shape { background: var(--pill); border: 1.5px solid var(--fg-dim); width: 60%; }

  .ctl { display: inline-flex; align-items: center; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }
  .rev { display: grid; place-items: center; width: 24px; height: 24px; border-radius: 50%; color: var(--fg-dim); }
  .rev:hover { background: var(--pill-hover); color: var(--fg); }
  .seg button:disabled { opacity: 0.45; }
  .seg small { font-size: 10px; opacity: 0.8; }
  .adv summary { list-style: none; cursor: default; }
  .adv summary h2 { display: inline; }
  .adv summary::before { content: '▸ '; color: var(--fg-dim); }
  .adv[open] summary::before { content: '▾ '; }
  .btn.plain { background: var(--pill); color: var(--fg); }
  .btn.danger { background: #d9383a; }
</style>
