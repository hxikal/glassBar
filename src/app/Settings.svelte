<script lang="ts">
  import { chargerTone, deviceTone } from '../lib/sound';
  import { availableMonitors, type Monitor } from '@tauri-apps/api/window';
  import { Bell, BellRing, Copy, DatabaseBackup, Gauge, HardDrive, Info, LayoutGrid, MapPin, Palette, PanelBottom, Power, Trash2, Users, Volume2 } from '@lucide/svelte';
  import { profile, store, update } from '../lib/config.svelte';
  import { events, ipc, type Config } from '../lib/ipc';
  import { ALERT_DEFAULTS, type DockAlertsCfg } from './dockAlert';
  /** The two sides of the dock, named for the edge it is on: screen-edge side, windows side. */
  const GAP_SIDES = { bottom: ['below', 'above'], top: ['above', 'below'], left: ['left of', 'right of'], right: ['right of', 'left of'] } as const;
  import LocationSettings from './LocationSettings.svelte';
  import WidgetManager from './widgets/WidgetManager.svelte';
  import About from './About.svelte';
  import AppearanceSettings from './AppearanceSettings.svelte';
  import PerformanceSettings from './PerformanceSettings.svelte';
  import StorageSettings from './StorageSettings.svelte';

  // The menu, in groups. Names are also the ?section= / openSettings() keys.
  type Item = readonly [name: string, icon: typeof Info, description: string];
  const MENU: readonly (readonly [group: string, items: readonly Item[]])[] = [
    ['Dock', [
      ['Bar', PanelBottom, 'Where the dock sits, how big it is and how it behaves.'],
      ['Appearance', Palette, 'Theme, design style, materials and colours.'],
      ['Widgets', LayoutGrid, 'Pick, order and size the widgets on your dock.'],
    ]],
    ['Alerts', [
      ['Notifications', Bell, 'Pop-up banners and Do Not Disturb.'],
      ['Dock alerts', BellRing, 'Short messages the dock shows by itself.'],
      ['Sounds', Volume2, 'Charger and USB device sounds.'],
    ]],
    ['System', [
      ['Performance', Gauge, 'What GlassBar uses, and how often widgets refresh.'],
      ['Storage', HardDrive, 'What GlassBar keeps on disk, and cleaning it up.'],
      ['Location', MapPin, 'Where you are, for weather and prayer times.'],
      ['Startup', Power, 'What happens when you sign in to Windows.'],
      ['Profiles', Users, 'Sets of widgets and dock settings you can switch between.'],
    ]],
    ['GlassBar', [
      ['Backup', DatabaseBackup, 'Save your settings to text, or bring them back.'],
      ['About', Info, 'Version, updates and what\'s new.'],
    ]],
  ];
  const items = MENU.flatMap(([, list]) => list);
  const sections = items.map(([s]) => s);
  import { SOURCE_WIDGET } from '../lib/notify';
  // Prayer time, Pomodoro and Timer are told by the dock itself (Settings > Dock alerts), not
  // by banners: not listed here (Rust: alerts::DOCK_SOURCES).
  const DOCK_SOURCES = ['solat', 'pomodoro', 'timer'];
  const BANNER_SOURCES = [...Object.entries(SOURCE_WIDGET).filter(([k]) => !DOCK_SOURCES.includes(k)).map(([k, v]) => [k, v.label] as const), ['update', 'Updates'] as const];
  type Section = string;
  // Updates now lives in About; old links to it still land there.
  const asSection = (s: string | null) => sections.find((x) => x === (s === 'Updates' ? 'About' : s));
  const page = $derived(items.find(([s]) => s === section)!);
  const PageIcon = $derived(page[1]);
  let mainEl: HTMLElement;
  function go(s: Section) { section = s; mainEl?.scrollTo(0, 0); }
  // Up/Down (and Home/End) move through the menu, like a list.
  function navKey(e: KeyboardEvent) {
    const i = sections.indexOf(section);
    const j = { ArrowDown: i + 1, ArrowUp: i - 1, Home: 0, End: sections.length - 1 }[e.key];
    if (j === undefined) return;
    e.preventDefault();
    const s = sections[(j + sections.length) % sections.length];
    go(s);
    (e.currentTarget as HTMLElement).closest('nav')?.querySelector<HTMLElement>(`[data-s="${s}"]`)?.focus();
  }
  // Opened on a section (e.g. from a widget's location label): ?section=… or, when already open, an event.
  let section = $state<Section>(asSection(new URLSearchParams(location.search).get('section')) ?? 'Bar');
  events.settingsSection((s) => { const x = asSection(s); if (x) section = x; });
  let monitors = $state<Monitor[]>([]);
  availableMonitors().then((m) => (monitors = m));

  const c = $derived(store.config!);
  const bar = $derived(profile().bar);
  const sides = $derived(GAP_SIDES[bar.edge]);
  const outer = $derived(bar.gap ?? 6);
  const inner = $derived(bar.gapInner ?? outer);
  // Length and spacing sliders: the dock follows while you drag (bar_preview moves only the
  // window, once per frame); the config is saved once on release.
  let live = $state<{ length?: number; gap?: number; inner?: number }>({});
  const lenNow = $derived(live.length ?? bar.length);
  const outerNow = $derived(live.gap ?? outer);
  const innerNow = $derived(live.inner ?? inner);
  let frame = 0;
  function preview(p: { length?: number; gap?: number; inner?: number }) {
    live = { ...live, ...p };
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => ipc.barPreview(lenNow, outerNow, innerNow).catch(() => {}));
  }
  function commit(f: (b: typeof bar) => void) {
    cancelAnimationFrame(frame);
    setBar(f).finally(() => (live = {}));
  }
  const set = (f: (c: Config) => void) => update(f);
  const setBar = (f: (b: Config['profiles'][string]['bar']) => void) => set((c) => f(c.profiles[c.activeProfile].bar));

  let newName = $state('');
  let exported = $state('');
  let importText = $state('');
  let message = $state('');

  function renameProfile(from: string, to: string) {
    to = to.trim();
    if (!to || to === from || c.profiles[to]) return;
    set((c) => {
      c.profiles = Object.fromEntries(Object.entries(c.profiles).map(([k, v]) => [k === from ? to : k, v]));
      if (c.activeProfile === from) c.activeProfile = to;
    });
  }

  function newProfile() {
    const name = newName.trim();
    if (!name || c.profiles[name]) return;
    set((c) => { c.profiles[name] = JSON.parse(JSON.stringify(c.profiles[c.activeProfile])); c.activeProfile = name; });
    newName = '';
  }

  async function doImport() {
    try { await ipc.importConfig(importText); message = 'Imported.'; importText = ''; }
    catch (e) { message = `Import failed: ${e}`; }
  }
</script>

<div class="settings">
  <nav aria-label="Settings sections">
    <div class="brand">Settings</div>
    {#each MENU as [group, list] (group)}
      <div class="ngroup" role="group" aria-labelledby="ng-{group}">
        <span class="nlabel" id="ng-{group}">{group}</span>
        {#each list as [s, Icon] (s)}
          <button class="nitem" class:sel={section === s} data-s={s} aria-current={section === s ? 'page' : undefined}
                  tabindex={section === s ? 0 : -1} onclick={() => go(s)} onkeydown={navKey}>
            <span class="nico"><Icon size={15} strokeWidth={2.2} /></span>{s}
          </button>
        {/each}
      </div>
    {/each}
  </nav>

  <main bind:this={mainEl}>
    <header class="phead">
      <span class="pico"><PageIcon size={22} /></span>
      <div><h1>{section}</h1><p>{page[2]}</p></div>
    </header>

    {#if section === 'Bar'}
      <h2>Placement</h2>
      <div class="group">
        <div class="field"><span>Position</span>
          <div class="seg">
            {#each ['top', 'bottom', 'left', 'right'] as const as e (e)}
              <button class:sel={!bar.floating && bar.edge === e} onclick={() => setBar((b) => { b.edge = e; b.floating = null; })}>{e}</button>
            {/each}
          </div>
        </div>
        <p class="hint">Or drag the bar anywhere: it snaps to the nearest edge, or floats if dropped mid-screen.</p>
        <div class="field"><span>Display</span>
          <select value={bar.allDisplays ? 'all' : (bar.monitor ?? '')} onchange={(e) => setBar((b) => {
            const v = e.currentTarget.value;
            b.allDisplays = v === 'all';
            b.monitor = v === '' || v === 'all' ? null : +v;
            b.floating = null;
          })}>
            <option value="">Main display</option>
            <option value="all">All displays</option>
            {#each monitors as m, i (i)}<option value={i}>{i + 1}: {m.name ?? 'Display'} ({m.size.width}×{m.size.height})</option>{/each}
          </select>
        </div>
      </div>
      <h2>Size</h2>
      <div class="group">
        <div class="field"><span>Height</span>
          <div class="seg">
            {#each ['compact', 'normal', 'large'] as const as s (s)}
              <button class:sel={bar.size === s} onclick={() => setBar((b) => (b.size = s))}>{s}</button>
            {/each}
          </div>
        </div>
        <div class="field"><span>Side width</span>
          <div class="seg">
            {#each [['slim', 'Slim'], ['regular', 'Regular'], ['wide', 'Wide']] as const as [w, label] (w)}
              <button class:sel={(bar.sideWidth ?? 'regular') === w} onclick={() => setBar((b) => (b.sideWidth = w))}>{label}</button>
            {/each}
          </div>
        </div>
        <p class="hint">How wide the bar is on the left or right edge ({bar.edge === 'left' || bar.edge === 'right' ? 'in use now' : 'used when you move it there'}). Wider shows labels next to the values.</p>
        <div class="field"><span>Length <b class="val">{Math.round(lenNow * 100)}%</b></span>
          <input type="range" min="0.2" max="1" step="0.01" value={lenNow} aria-label="Dock length"
                 oninput={(e) => preview({ length: +e.currentTarget.value })} onchange={(e) => { const v = +e.currentTarget.value; commit((b) => (b.length = v)); }} />
        </div>
      </div>
      <h2>Spacing</h2>
      <div class="group">
        <div class="field"><span>Space {sides[0]} the dock <b class="val">{outerNow}px</b></span>
          <input type="range" min="0" max="48" step="1" value={outerNow} aria-label="Space between the dock and the screen edge"
                 oninput={(e) => preview({ gap: +e.currentTarget.value, inner })} onchange={(e) => { const v = +e.currentTarget.value; commit((b) => { b.gapInner = inner; b.gap = v; }); }} />
        </div>
        <div class="field"><span>Space {sides[1]} the dock <b class="val">{innerNow}px</b></span>
          <input type="range" min="0" max="48" step="1" value={innerNow} aria-label="Space between the dock and your windows"
                 oninput={(e) => preview({ inner: +e.currentTarget.value })} onchange={(e) => { const v = +e.currentTarget.value; commit((b) => (b.gapInner = v)); }} />
        </div>
        <p class="hint">The first is towards the screen edge, the second towards your windows (with "Reserve screen space" on, maximised windows stop there). They follow the dock to any edge.</p>
      </div>
      <h2>Behaviour</h2>
      <div class="group">
        <label class="field"><span>Auto-hide</span>
          <input type="checkbox" checked={bar.autoHide} onchange={(e) => setBar((b) => { b.autoHide = e.currentTarget.checked; b.floating = null; })} />
        </label>
        <p class="hint">The bar slides away and comes back when you touch that screen edge. Tip: drag the bar past the screen edge to turn this on, and drag it back in to turn it off.</p>
        <label class="field"><span>Reserve screen space</span>
          <input type="checkbox" checked={bar.reserveSpace} onchange={(e) => setBar((b) => (b.reserveSpace = e.currentTarget.checked))} />
        </label>
        <p class="hint">Maximized windows stop at the bar instead of covering it.</p>
      </div>
      <h2>Windows taskbar</h2>
      <div class="group">
        <div class="field"><span>The Windows taskbar while GlassBar runs</span>
          <select value={bar.nativeTaskbar} onchange={(e) => setBar((b) => (b.nativeTaskbar = e.currentTarget.value as typeof b.nativeTaskbar))}>
            <option value="show">Leave as is</option>
            <option value="autohide">Auto-hide</option>
            <option value="hidden">Hide completely</option>
          </select>
        </div>
        <p class="hint">Restored when GlassBar quits. Also restored when you sign out. If GlassBar is force-killed, start it again and choose tray → "Restore Windows taskbar".</p>
      </div>

    {:else if section === 'Appearance'}
      <AppearanceSettings />

    {:else if section === 'Performance'}
      <PerformanceSettings />

    {:else if section === 'Storage'}
      <StorageSettings />

    {:else if section === 'Widgets'}
      <WidgetManager />

    {:else if section === 'Location'}
      <LocationSettings />

    {:else if section === 'Profiles'}
      <h2>Your profiles</h2>
      <div class="group">
        {#each Object.keys(c.profiles) as name (name)}
          <div class="wrow">
            <input type="radio" name="active" checked={c.activeProfile === name} onchange={() => set((c) => (c.activeProfile = name))} aria-label="Use {name}" />
            <input class="grow" value={name} onchange={(e) => renameProfile(name, e.currentTarget.value)} aria-label="Profile name" />
            <span class="hint">{c.profiles[name].widgets.length} widgets</span>
            <button class="icon-btn" disabled={Object.keys(c.profiles).length < 2}
                    onclick={() => set((c) => { delete c.profiles[name]; if (c.activeProfile === name) c.activeProfile = Object.keys(c.profiles)[0]; })}
                    aria-label="Delete {name}"><Trash2 size={14} /></button>
          </div>
        {/each}
      </div>
      <h2>New profile</h2>
      <div class="wrow">
        <input class="grow" placeholder="New profile name, e.g. Gaming" bind:value={newName} onkeydown={(e) => e.key === 'Enter' && newProfile()} />
        <button class="btn" onclick={newProfile}><Copy size={14} /> Duplicate current</button>
      </div>
      <p class="hint">Switch profiles from here or from the tray icon menu.</p>

    {:else if section === 'Notifications'}
      <h2>Do Not Disturb</h2>
      <div class="group">
        <label class="field"><span>Do Not Disturb</span>
          <input type="checkbox" checked={c.dnd} onchange={(e) => set((c) => (c.dnd = e.currentTarget.checked))} />
        </label>
        <p class="hint">On: no banners, toasts or sounds. Everything still goes into the Notification Center list.</p>
      </div>
      <h2>Pop-up banners</h2>
      <div class="group">
        <label class="field"><span>Show pop-up banners</span>
          <input type="checkbox" checked={c.banners.show} onchange={(e) => set((c) => (c.banners.show = e.currentTarget.checked))} />
        </label>
        <p class="hint">A small banner next to the dock (timer done, prayer time, download finished…). Click it to jump to the widget. Off: Windows shows its own notification instead.</p>
        {#if c.banners.show}
          <div class="field"><span>Display time</span>
            <span class="segs">{#each [3, 5, 8, 10] as s (s)}<button class="btn" class:on={c.banners.seconds === s} onclick={() => set((c) => (c.banners.seconds = s as 3 | 5 | 8 | 10))}>{s} s</button>{/each}</span>
          </div>
          <div class="field"><span>Position</span>
            <span class="segs"><button class="btn" class:on={c.banners.position === 'auto'} onclick={() => set((c) => (c.banners.position = 'auto'))}>Next to the dock</button>
            <button class="btn" class:on={c.banners.position === 'corner'} onclick={() => set((c) => (c.banners.position = 'corner'))}>Screen corner</button></span>
          </div>
          <div class="field"><span>On click</span>
            <span class="segs"><button class="btn" class:on={c.banners.onClick === 'highlight'} onclick={() => set((c) => (c.banners.onClick = 'highlight'))}>Show the widget</button>
            <button class="btn" class:on={c.banners.onClick === 'open'} onclick={() => set((c) => (c.banners.onClick = 'open'))}>…and open its panel</button></span>
          </div>
          <label class="field"><span>Play the alert sound with the banner</span>
            <input type="checkbox" checked={c.banners.sound} onchange={(e) => set((c) => (c.banners.sound = e.currentTarget.checked))} />
          </label>
        {/if}
      </div>
      {#if c.banners.show}
        <h2>Banners per widget</h2>
        <div class="group">
          {#each BANNER_SOURCES as [k, label] (k)}
            <label class="field"><span>{label}</span>
              <input type="checkbox" checked={!c.banners.muted.includes(k)}
                     onchange={(e) => set((c) => (c.banners.muted = e.currentTarget.checked ? c.banners.muted.filter((m) => m !== k) : [...c.banners.muted, k]))} />
            </label>
          {/each}
          <p class="hint">Off: no banner for that widget (it still goes into the list).</p>
        </div>
      {/if}

    {:else if section === 'Dock alerts'}
      {@const da = { ...ALERT_DEFAULTS, ...(c.dockAlerts ?? {}) }}
      {@const setDa = (p: Partial<DockAlertsCfg>) => set((c) => (c.dockAlerts = { ...da, ...p }))}
      <p class="hint">For a moment the whole dock shows one thing, then goes back to your widgets. Hover to keep it, click to dismiss. Not over full-screen apps, except prayer time. No notification.</p>
      <h2>What the dock tells you</h2>
      <div class="group">
        {#each [
          ['charger', 'Charger', 'The charge level when you plug the charger in or unplug it.'],
          ['volume', 'Volume', 'When the volume changes outside GlassBar: keyboard keys, Windows or another app.'],
          ['brightness', 'Brightness', "When the built-in screen's brightness changes outside GlassBar. External monitors don't report it. If Windows changes it by itself (auto brightness), you'll see that too."],
          ['prayer', 'Prayer time', 'The prayer name and time when it begins (Waktu Solat widget, with its alerts on).'],
          ['pomodoro', 'Pomodoro', 'When a focus session ends (break time) and when a break ends (back to focus), with Start / Pause / Skip.'],
          ['timer', 'Timer', "When a timer runs out, with +1 min and +5 min."],
          ['water', 'Water reminder', 'Time to drink, with buttons to log a cup or snooze (Water widget, with reminders on).'],
        ] as const as [k, label, hint] (k)}
          <div class="field"><span>{label}</span>
            <span class="row"><button class="btn ghost" onclick={() => ipc.dockAlertTest(k)}>Test</button>
            {#if k === 'charger'}
              <input type="checkbox" aria-label={label} checked={c.chargerFlash} onchange={(e) => set((c) => (c.chargerFlash = e.currentTarget.checked))} />
            {:else}
              <input type="checkbox" aria-label={label} checked={da[k]} onchange={(e) => setDa({ [k]: e.currentTarget.checked })} />
            {/if}</span>
          </div>
          <p class="hint">{hint}</p>
        {/each}
      </div>
      <h2>Options</h2>
      <div class="group">
        <label class="field"><span>Hide Windows' own volume and brightness pop-up</span>
          <input type="checkbox" checked={da.hideWindowsOsd} disabled={!da.volume && !da.brightness} onchange={(e) => setDa({ hideWindowsOsd: e.currentTarget.checked })} />
        </label>
        <p class="hint">So only GlassBar's shows, not both. Works while the Volume or Brightness alert above is on; Windows' pop-up comes back when you close GlassBar. Right after GlassBar starts, the very first one may still flash once. It also hides Windows' media pop-up (play/pause keys).</p>
        <label class="field"><span>Show when the dock is hidden</span>
          <input type="checkbox" checked={da.reveal} onchange={(e) => setDa({ reveal: e.currentTarget.checked })} />
        </label>
        <p class="hint">With auto-hide on, the dock slides out for the alert and away again after.</p>
        <div class="field"><span>How long</span>
          <div class="seg">
            {#each [['short', 'Short'], ['normal', 'Normal'], ['long', 'Long']] as const as [v, label] (v)}
              <button class:sel={da.length === v} onclick={() => setDa({ length: v })}>{label}</button>
            {/each}
          </div>
        </div>
      </div>

    {:else if section === 'Sounds'}
      <h2>Charger</h2>
      <div class="group">
        <label class="field"><span>Charger sounds (plugged in / unplugged)</span>
          <input type="checkbox" checked={c.deviceSounds.charger} onchange={(e) => set((c) => (c.deviceSounds.charger = e.currentTarget.checked))} />
        </label>
        <div class="field"><span>Preview</span>
          <span><button class="btn" onclick={() => chargerTone(true, c.deviceSounds.volume)}>Plugged in</button>
          <button class="btn" onclick={() => chargerTone(false, c.deviceSounds.volume)}>Unplugged</button></span>
        </div>
        <p class="hint">Plays only when the charger state really changes (Windows power events), never repeatedly while it stays plugged in.</p>
      </div>
      <h2>USB devices</h2>
      <div class="group">
        <label class="field"><span>Sound when a USB device is connected</span>
          <input type="checkbox" checked={c.deviceSounds.connect} onchange={(e) => set((c) => (c.deviceSounds.connect = e.currentTarget.checked))} />
        </label>
        <label class="field"><span>Sound when a USB device is removed</span>
          <input type="checkbox" checked={c.deviceSounds.disconnect} onchange={(e) => set((c) => (c.deviceSounds.disconnect = e.currentTarget.checked))} />
        </label>
        <div class="field"><span>Volume <b class="val">{Math.round(c.deviceSounds.volume * 100)}%</b></span>
          <input type="range" min="0.05" max="1" step="0.05" value={c.deviceSounds.volume} onchange={(e) => set((c) => (c.deviceSounds.volume = +e.currentTarget.value))} />
        </div>
        <div class="field"><span>Preview</span>
          <span><button class="btn" onclick={() => deviceTone(true, c.deviceSounds.volume)}>Connect</button>
          <button class="btn" onclick={() => deviceTone(false, c.deviceSounds.volume)}>Disconnect</button></span>
        </div>
        <p class="hint">Uses Windows' USB device notifications (no polling); quick bursts play once. Windows also plays its own device sounds unless you turn them off in Control Panel › Sound › Sounds. Silent while Do Not Disturb is on. Independent of timer, Pomodoro and prayer sounds.</p>
      </div>

    {:else if section === 'Startup'}
      <div class="group">
        <label class="field"><span>Launch GlassBar when I sign in</span>
          <input type="checkbox" checked={c.startup.launchAtLogin} onchange={(e) => set((c) => (c.startup.launchAtLogin = e.currentTarget.checked))} />
        </label>
      </div>

    {:else if section === 'Backup'}
      <h2>Export</h2>
      <div class="group">
        <div class="wrow">
          <button class="btn" onclick={async () => (exported = await ipc.exportConfig())}>Export configuration</button>
          {#if exported}<button class="btn" onclick={() => navigator.clipboard.writeText(exported).then(() => (message = 'Copied to clipboard.'))}>Copy</button>{/if}
        </div>
        {#if exported}<textarea readonly rows="10">{exported}</textarea>{/if}
      </div>
      <h2>Import</h2>
      <div class="group">
        <textarea rows="8" placeholder="Paste exported JSON here" bind:value={importText}></textarea>
        <button class="btn" disabled={!importText.trim()} onclick={doImport}>Import and apply</button>
      </div>
      {#if message}<p class="hint">{message}</p>{/if}

    {:else if section === 'About'}
      <About />
    {/if}
  </main>
</div>

<style>
  /* Settings colours come from the Appearance theme: --tint is the theme's (or your custom)
     background, so the page, its cards and the menu follow it; the menu also takes a little
     of the accent so it reads as its own surface. */
  :global(html[data-window='settings']) {
    user-select: text;
    --settings-bg: rgb(var(--tint));
    --panel: color-mix(in srgb, rgb(var(--tint)), #fff 6%);
    --side-bg: color-mix(in srgb, var(--accent) 10%, color-mix(in srgb, rgb(var(--tint)), #000 34%));
    --side-sel: color-mix(in srgb, var(--accent) 24%, transparent);
    --field-bg: color-mix(in srgb, rgb(var(--tint)), #000 22%);
  }
  :global(html[data-window='settings'][data-theme='light']) {
    --panel: color-mix(in srgb, rgb(var(--tint)), #fff 85%);
    --side-bg: color-mix(in srgb, var(--accent) 9%, color-mix(in srgb, rgb(var(--tint)), #000 5%));
    --side-sel: color-mix(in srgb, var(--accent) 16%, transparent);
    --field-bg: rgb(var(--tint));
  }
  :global(html[data-window='settings'] body) { background: var(--settings-bg); }

  .settings { display: flex; height: 100%; }

  /* ---- Menu ---- */
  nav {
    width: 214px; flex: none; overflow-y: auto; padding: 14px 10px 14px;
    display: flex; flex-direction: column; gap: 10px;
    background: var(--side-bg); border-right: 1px solid var(--line-soft);
  }
  .brand { font-weight: 600; font-size: 17px; padding: 2px 10px 0; letter-spacing: -0.01em; }
  .ngroup { display: flex; flex-direction: column; gap: 1px; }
  .nlabel { font-size: 11.5px; font-weight: 600; color: var(--fg-3); padding: 0 10px 3px; }
  .nitem {
    display: flex; align-items: center; gap: 10px; text-align: left;
    padding: 4px 10px 4px 6px; border-radius: calc(10px * var(--rs)); color: var(--fg-dim);
  }
  .nitem:hover { background: var(--pill-hover); color: var(--fg); }
  .nitem:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .nico {
    display: grid; place-items: center; width: 26px; height: 26px; flex: none;
    border-radius: calc(8px * var(--rs)); background: var(--pill); color: var(--fg);
  }
  .nitem.sel { background: var(--side-sel); color: var(--fg); font-weight: 600; }
  .nitem.sel .nico { background: var(--accent); color: var(--on-accent); }
  /* Glass themes: the menu takes a soft wash of the accent, like light through glass. */
  :global(html[data-window='settings'][data-preset='glass']) nav,
  :global(html[data-window='settings'][data-preset='liquid']) nav {
    background:
      radial-gradient(120% 50% at 0% 0%, color-mix(in srgb, var(--accent) 22%, transparent), transparent 70%),
      var(--side-bg);
  }
  /* Liquid Glass: the chosen item is a white lens with the accent icon, like its "on" tiles. */
  :global(html[data-window='settings'][data-preset='liquid']) .nitem.sel { background: color-mix(in srgb, #fff 16%, transparent); box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.35); }
  :global(html[data-window='settings'][data-preset='liquid']) .nitem.sel .nico { background: #fff; color: var(--accent-ink); }

  /* ---- Page ---- */
  main { flex: 1; min-width: 0; overflow: auto; padding: 18px 32px 48px; }
  main > :global(*) { max-width: 760px; }
  .phead { display: flex; align-items: center; gap: 14px; margin: 4px 0 6px; }
  .pico {
    display: grid; place-items: center; width: 44px; height: 44px; flex: none;
    border-radius: calc(12px * var(--rs)); background: var(--accent); color: var(--on-accent);
  }
  .phead p { margin: 2px 0 0; color: var(--fg-dim); font-size: 13px; }
  :global(html[data-theme='light'] .settings .hint.warn) { color: #b25e00; }
  /* Used by widget panels in every window too. */
  :global(.btn) { display: inline-flex; align-items: center; gap: 6px; padding: 7px 14px; border-radius: calc(9px * var(--rs)); background: var(--accent); color: var(--on-accent); }
  .btn.ghost { background: var(--pill); color: var(--fg); padding: 5px 12px; }
  .btn.ghost:hover { background: var(--pill-hover); }
  .row { display: inline-flex; align-items: center; gap: 10px; }
  :global(.btn:disabled) { opacity: 0.5; }
  /* Shared by every section component (Location, About, widget manager…). */
  .settings :global {
    h1 { font-size: 22px; font-weight: 600; margin: 0; letter-spacing: -0.01em; }
    h2 { font-size: 13px; font-weight: 600; color: var(--fg-dim); margin: 24px 0 8px 4px; }
    .group {
      background: var(--panel); border-radius: calc(14px * var(--rs)); padding: 4px 18px; margin-bottom: 14px;
      display: flex; flex-direction: column; gap: 0; border: 1px solid var(--line-soft);
    }
    .field { display: flex; align-items: center; justify-content: space-between; gap: 20px; padding: 12px 0; min-height: 48px; }
    .field > span:first-child { font-weight: 500; line-height: 1.35; }
    .group > .field ~ .field { border-top: 1px solid var(--line-soft); }
    .field.off { opacity: 0.5; }
    .field input[type='range'] { width: 220px; accent-color: var(--accent); }
    input:where([type='range'], [type='radio']) { accent-color: var(--accent); }
    .wrow .hint { margin: 0; }
    /* Slider values: a small badge next to the label instead of loose text. */
    .val {
      display: inline-block; margin-left: 6px; padding: 1px 8px; border-radius: var(--r-chip);
      font-size: 12px; font-weight: 600; font-variant-numeric: tabular-nums;
      background: color-mix(in srgb, var(--accent) 18%, transparent); color: var(--fg);
    }
    .hint { font-size: 12.5px; line-height: 1.45; color: var(--fg-dim); margin: -6px 0 10px; max-width: 68ch; }
    .hint.warn { color: #ff9f0a; }
    main > .hint { margin: 10px 4px 12px; }
    .seg { display: flex; gap: 2px; padding: 3px; border-radius: calc(10px * var(--rs)); background: var(--field-bg); border: 1px solid var(--line-soft); }
    .seg button { padding: 5px 12px; border-radius: calc(8px * var(--rs)); text-transform: capitalize; color: var(--fg-dim); }
    .seg button:hover:not(.sel):not(:disabled) { color: var(--fg); background: var(--pill-hover); }
    .seg .sel { background: var(--accent); color: var(--on-accent); font-weight: 500; }
    :where(select, input:not([type]), textarea, .grow) { padding: 7px 10px; border-radius: calc(8px * var(--rs)); border: 1px solid var(--line-soft); background: var(--field-bg); }
    :where(button, select, input, textarea, summary):focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
    textarea { width: 100%; font-family: Consolas, monospace; font-size: 12px; resize: vertical; margin: 10px 0; }
    /* Choice buttons (Notifications): plain, the chosen one in the accent colour. */
    .segs { display: inline-flex; gap: 4px; flex-wrap: wrap; justify-content: flex-end; }
    .segs .btn { background: var(--field-bg); color: var(--fg); padding: 6px 10px; font-size: 12px; border: 1px solid var(--line-soft); }
    .segs .btn.on { background: var(--accent); color: var(--on-accent); border-color: transparent; }
    .wrow { display: flex; align-items: center; gap: 8px; padding: 6px 0; }
    .wrow b { flex: 1; font-weight: 500; }
    .grow { flex: 1; }
    .wico { display: grid; place-items: center; width: 28px; height: 28px; border-radius: calc(8px * var(--rs)); background: var(--accent); color: var(--on-accent); flex: none; }
    .wform { padding: 4px 0 12px 36px; display: flex; flex-direction: column; gap: 8px; }
    .wform label { display: flex; align-items: center; gap: 8px; }
    .gallery { display: grid; grid-template-columns: repeat(auto-fill, minmax(190px, 1fr)); gap: 10px; }
    .gcard { display: flex; flex-direction: column; align-items: flex-start; gap: 6px; padding: 14px; text-align: left; border-radius: calc(14px * var(--rs)); background: var(--panel); border: 1px solid var(--line-soft); transition: transform 0.3s var(--spring); }
    .gcard:hover { transform: translateY(-2px); }
    .gcard .hint { margin: 0; }
  }
</style>
