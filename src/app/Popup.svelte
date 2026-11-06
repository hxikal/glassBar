<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { profile } from '../lib/config.svelte';
  import { events, ipc } from '../lib/ipc';
  import { glass } from '../lib/liquid';
  import { GALLERY } from '../lib/popup';
  import { settingsOf, widgets } from '../widgets/registry';
  import Gallery from './Gallery.svelte';
  import CapMenu from '../widgets/ocr/HoverMenu.svelte';
  import { parseCapMenu } from '../widgets/ocr/menu';
  import WeatherPanel from '../widgets/weather/Popup.svelte';
  import SoundPanel from '../widgets/sound/SoundPanel.svelte';
  import { BATTERY_PANEL, SOUND_PANEL, soundOf } from '../widgets/sound/settings';
  import BatterySection from '../widgets/battery/BatterySection.svelte';
  import { parsePartPanel } from '../widgets/productivity/parts';
  import { updateWidgetSettings } from '../lib/config.svelte';
  import WindowPreviews from './WindowPreviews.svelte';
  import FolderPanel from '../widgets/folders/FolderPanel.svelte';
  import { parseFolderKey } from '../widgets/folders/folders';
  import Feedback from '../widgets/ocr/Feedback.svelte';
  import type { CaptureDone } from '../lib/ipc';
  import SearchPanel from '../widgets/start/SearchPanel.svelte';
  import WhatsNew from './WhatsNew.svelte';
  /** "__whatsnew:<version>": the card after an update. */
  const WHATS_NEW = '__whatsnew:';
  import { SEARCH_PANEL } from '../widgets/start/search';

  // Which widget's panel is showing. Cleared when Rust hides the popup, so hidden
  // panels stop their timers. (Rust sends hide before the next show, so no race.)
  let instance = $state<string | null>(null);
  /** "__feedback": the card shown right after a screenshot / OCR snap. */
  const FEEDBACK = '__feedback';
  let feedback = $state<CaptureDone | null>(null);
  const close = () => ipc.popupHide();
  // Panels grow out of the dock: scale from the side that faces it.
  const ORIGIN = { top: 'top center', bottom: 'bottom center', left: 'left center', right: 'right center' } as const;
  const WINDOWS = '__windows:';
  /** "__hover:<instance>" = that widget's panel opened by hovering (closes on mouse-out). */
  const HOVER = '__hover:';
  /** "__weather:<clock instance>" = the weather forecast, opened from the clock's weather part. */
  const WEATHER = '__weather:';
  const capMenu = $derived(parseCapMenu(instance));
  const folder = $derived(parseFolderKey(instance));
  const hover = $derived(!!instance && (instance.startsWith(WINDOWS) || instance.startsWith(HOVER) || !!capMenu || !!folder?.hover));
  const target = $derived(instance?.startsWith(HOVER) ? instance.slice(HOVER.length) : instance);
  const w = $derived(target ? profile().widgets.find((x) => x.instance === target) : undefined);
  const m = $derived(w ? widgets[w.id] : undefined);

  // Hover popups close shortly after the pointer leaves both the icon and the popup.
  let leaveTimer: ReturnType<typeof setTimeout>;
  const leaveSoon = () => { clearTimeout(leaveTimer); if (hover) leaveTimer = setTimeout(() => ipc.popupLeaveClose(), 150); };
  const stay = () => clearTimeout(leaveTimer);

  // Size the window to the panel's content (no empty space below, no needless scrollbar):
  // measure the content whenever it changes size and ask Rust to fit the popup to it.
  let surface = $state<HTMLElement>();
  let content = $state<HTMLElement>();
  let lastFit = 0;
  /** Fit again from scratch: every open resets the window to its initial size. */
  let refit: (() => void) | null = null;
  $effect(() => {
    if (!content || !surface || !instance) return;
    const cs = getComputedStyle(surface);
    const chrome = ['paddingTop', 'paddingBottom', 'borderTopWidth', 'borderBottomWidth'].reduce((s, k) => s + parseFloat(cs[k as any] as string), 0);
    const sides = parseFloat(cs.borderLeftWidth) + parseFloat(cs.borderRightWidth);
    const fit = () => {
      // The exact (fractional) height, rounded up: offsetHeight rounds, and a panel a fraction
      // too short shows a scrollbar, which reflows it, which removes the scrollbar (a jiggle).
      // The computed height, unlike the on-screen box, ignores the opening scale animation.
      const h = Math.ceil(parseFloat(getComputedStyle(content!).height) + chrome);
      // While a scrollbar shows, the content is narrower than it will be without one, so its
      // height is off (a screenshot thumbnail gets shorter, text gets taller). Shrinking to that
      // removed the scrollbar, which changed the height back: the window flipped between two
      // sizes every frame and the card looked doubled. So only grow then; once the scrollbar
      // is gone the content widens, this runs again and measures it true.
      if (h < lastFit && surface!.offsetWidth - surface!.clientWidth > sides + 4) return;
      if (Math.abs(h - lastFit) > 1) { lastFit = h; ipc.popupFit(h); }
    };
    refit = () => { lastFit = 0; fit(); };
    refit();
    const ro = new ResizeObserver(fit);
    ro.observe(content);
    return () => ro.disconnect();
  });

  onMount(() => {
    const offs = [
      events.popupShow(async (i) => {
        instance = i; stay();
        // Reopening the same panel keeps this page as it was: fit it again all the same.
        await tick();
        refit?.();
        if (i !== FEEDBACK) return;
        // Draw the card (and decode its thumbnail) before the window appears.
        await tick();
        const t = (feedback as { thumb?: string } | null)?.thumb;
        if (t) { const pic = new Image(); pic.src = t; await pic.decode().catch(() => {}); }
        setTimeout(() => ipc.popupReveal(), 16);
      }),
      events.popupHide(() => { instance = null; }),
      events.popupLeave(leaveSoon),
      events.captureDone((d) => (feedback = d)),
    ];
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && close();
    addEventListener('keydown', onKey);
    return () => { offs.forEach((p) => p.then((off) => off())); removeEventListener('keydown', onKey); };
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="surface popup" use:glass={true} class:hover bind:this={surface} onpointerenter={stay} onpointerleave={leaveSoon}>
  {#key instance}
  <div class="content" bind:this={content} style:--origin={ORIGIN[profile().bar.edge]}>
  {#if instance?.startsWith(WINDOWS)}
    {#key instance}<WindowPreviews appKey={instance.slice(WINDOWS.length)} {close} />{/key}
  {:else if capMenu}
    {@const cw = profile().widgets.find((x) => x.instance === capMenu.instance)}
    {#if cw}<CapMenu tab={capMenu.tab} instance={cw.instance} settings={settingsOf(widgets.ocr, cw)} />{/if}
  {:else if instance === FEEDBACK && feedback}
    {#key feedback}<Feedback data={feedback} {close} />{/key}
  {:else if folder}
    {#key instance}<FolderPanel instance={folder.instance} id={folder.id} {close} />{/key}
  {:else if instance && parsePartPanel(instance)}
    {@const pp = parsePartPanel(instance)!}
    {@const pw = profile().widgets.find((x) => x.instance === pp.instance)}
    {#if pw}
      {#key instance}<pp.part.Popup instance={pw.instance} settings={settingsOf(widgets.productivity, pw)} {close} />{/key}
    {/if}
  {:else if instance?.startsWith(BATTERY_PANEL)}
    {@const qs = profile().widgets.find((x) => x.instance === instance!.slice(BATTERY_PANEL.length))}
    {#if qs}
      <div class="solo"><BatterySection instance={qs.instance} style={(qs.settings as { batteryStyle?: 'classic' | 'liquid' | 'fill' })?.batteryStyle ?? 'classic'} /></div>
    {/if}
  {:else if instance?.startsWith(SOUND_PANEL)}
    {@const qs = profile().widgets.find((x) => x.instance === instance!.slice(SOUND_PANEL.length))}
    {#if qs}
      {@const snd = soundOf(qs.settings as never)}
      <SoundPanel settings={snd} save={(p) => updateWidgetSettings(qs.instance, { sound: { ...snd, ...p } })} />
    {/if}
  {:else if instance?.startsWith(WHATS_NEW)}
    <WhatsNew version={instance.slice(WHATS_NEW.length)} {close} />
  {:else if instance?.startsWith(SEARCH_PANEL)}
    <SearchPanel {close} />
  {:else if instance?.startsWith(WEATHER)}
    <WeatherPanel />
  {:else if instance === GALLERY}
    <Gallery {close} />
  {:else if w && m?.Popup}
    {#key instance}
      <m.Popup.component instance={w.instance} settings={settingsOf(m, w)} {close} />
    {/key}
  {/if}
  </div>
  {/key}
</div>

<style>
  /* --pad: how far a panel's full-background (the water, the battery Fill) reaches past its
     content to the window's edges (lib/Tank.svelte). */
  .popup { --pad: 14px; padding: var(--pad); overflow: auto; }
  /* flow-root: a last child's bottom margin stays inside, so the measured height is exact */
  .content { display: flow-root; }
  /* Opening: 180 ms ease-out, 0.96 → 1 and a fade, from the dock side (content mounts on every show). */
  .content { animation: grow 180ms cubic-bezier(0.2, 0, 0, 1) both; transform-origin: var(--origin); }
  @keyframes grow { from { opacity: 0; transform: scale(0.96); } }
  /* Liquid Glass: a softer, slightly longer grow out of the dock. */
  :global(:root[data-preset='liquid']) .content { animation: liquid-grow 220ms cubic-bezier(0.2, 0.8, 0.2, 1) both; }
  @keyframes liquid-grow { from { opacity: 0; transform: scale(0.94); filter: blur(4px); } }
  @media (prefers-reduced-motion: reduce) { .content { animation: none; } }
  .popup.hover { --pad: 8px; }
  /* A section shown as its own panel: no gap above its heading. */
  .solo :global(.bhead) { margin-top: 0; }
  /* Content above a full-background fill (battery Fill design). */
  .solo > :global(:not(.tank)) { position: relative; z-index: 1; }
</style>
