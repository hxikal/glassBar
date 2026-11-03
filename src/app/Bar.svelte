<script lang="ts">
  import { availableMonitors, getCurrentWindow, primaryMonitor } from '@tauri-apps/api/window';
  import { flip } from 'svelte/animate';
  import { onMount } from 'svelte';
  import { adhan, setAdhanPlaying, stopAdhan } from '../lib/adhan.svelte';
  import { Check, ChevronDown, ChevronLeft, ChevronRight, ChevronUp, Coffee, Droplet, Maximize2, MoonStar, Pause, Play, Plus, SkipForward, Square, Target, Timer as TimerIcon, PlugZap, Sun, Unplug, Volume2, VolumeX, X } from '@lucide/svelte';
  import { profile, store, update } from '../lib/config.svelte';
  import { events, ipc, type WidgetInst, type WidgetSize } from '../lib/ipc';
  import { isMainDock } from '../lib/dock';
  import { FLASH } from './chargeFlash';
  import { ALERT_DEFAULTS, arrive, holdMs, viewOf, type AlertKind, type DockAlert } from './dockAlert';
  import { fmtWater } from '../widgets/water/units';
  import { hiddenCounts, insertIndex, revealBy, snap } from '../lib/geometry';
  import { GALLERY, openPopup } from '../lib/popup';
  import { adhanOf, chargerTone, chime, deviceTone, playAdzan, shutter, tick } from '../lib/sound';
  import { showMenu } from '../lib/menu';
  import { settingsOf, widgets } from '../widgets/registry';
  import { visibleOnDock } from '../widgets/layout';
  import { jumpTarget } from '../lib/notify';
  import { PARTS, partPanel } from '../widgets/productivity/parts';
  import { livePomo } from '../widgets/pomodoro/pomo.svelte';
  import { tickingNow } from '../lib/now.svelte';
  import FocusDock from './FocusDock.svelte';
  import { focusDock, focusShown } from './focusDock';

  const win = getCurrentWindow();
  let edit = $state(false);
  let scroller: HTMLDivElement;

  const bar = $derived(profile().bar);
  const vertical = $derived(bar.edge === 'left' || bar.edge === 'right');
  // Widgets that are on; the standalone Start & Search hides while the App Launcher shows it inline.
  const placed = $derived(visibleOnDock(profile().widgets, widgets));

  // While dragging a widget in edit mode we show a local order, committed on drop.
  let order = $state<string[] | null>(null);
  const shown = $derived(order ? order.map((id) => placed.find((w) => w.instance === id)!).filter(Boolean) : placed);
  // Pomodoro "Dim other dock widgets while focusing": every widget but Productivity fades.
  const pomo = livePomo();
  const focusDim = $derived(!!pomo.state?.cfg.dimOthers && pomo.running && pomo.state?.pomo.phase === 'work');

  // ---- Focus on the dock ----
  // During a focus session the whole dock shows it (FocusDock.svelte). With the pointer on
  // the dock the widgets come back (after 250 ms, so passing through doesn't flicker; never
  // from the focus controls themselves); leaving it brings the focus view back after 700 ms,
  // but not while a panel or menu opened from the dock is still open. On by default
  // (Pomodoro settings: "Show focus across the dock").
  const focusOn = $derived((profile().widgets.find((w) => w.id === 'productivity')?.settings as { focusDock?: boolean } | undefined)?.focusDock !== false);
  const fNow = tickingNow(() => focusOn && pomo.running);
  const focus = $derived(focusOn ? focusDock(pomo.state, fNow.now) : null);
  let peek = $state(false);
  let hovering = false;
  let panelOpen = false;
  let peekIn: ReturnType<typeof setTimeout> | undefined;
  let peekOut: ReturnType<typeof setTimeout> | undefined;
  // A session that starts while the pointer is on the dock (Start was clicked here) waits
  // for the pointer to leave before covering the widgets.
  let wasActive = false;
  $effect(() => {
    const active = !!focus;
    if (active && !wasActive) peek = hovering;
    wasActive = active;
  });
  function focusMove(e: PointerEvent) {
    hovering = true;
    clearTimeout(peekOut);
    if (!focus || peek) return;
    if ((e.target as Element).closest('.focus-acts')) { clearTimeout(peekIn); peekIn = undefined; return; }
    peekIn ??= setTimeout(() => { peek = true; peekIn = undefined; }, 250);
  }
  function focusLeave() {
    hovering = false;
    clearTimeout(peekIn); peekIn = undefined;
    if (!peek) return;
    clearTimeout(peekOut);
    peekOut = setTimeout(async function back() {
      if (hovering) return;
      if (panelOpen || (await ipc.menuOpen().catch(() => false))) { peekOut = setTimeout(back, 400); return; }
      peek = false;
    }, 700);
  }

  // ---- Auto-hide ----
  // Showing and hiding are decided in Rust from the real mouse position (bar.rs
  // watch_autohide): page enter/leave events go missing when the bar moves under a still
  // pointer or a menu is open. Here only: slide in at once on hover, and stay out while editing.
  const autoHide = $derived(bar.autoHide && !bar.floating);
  const setBar = (f: (b: typeof bar) => void) => update((c) => f(c.profiles[c.activeProfile].bar));
  function onEnter() { if (autoHide) ipc.barReveal(true); }
  $effect(() => { if (autoHide && edit) ipc.barReveal(true); });

  // ---- Dock alerts ----
  // For a moment the whole dock says one thing: the charger moved, the volume or brightness
  // changed from outside GlassBar, prayer time (rules and wording in dockAlert.ts). Rust
  // decides whether an event fires at all; here we only gate on the settings and time it.
  // Timeouts own the schedule, never a transitionend: with motion off there is nothing to
  // wait for. The widgets stay mounted and fade, because unmounting them mid-animation makes
  // the leaving ones float over their neighbours (see the App Launcher's +N row).
  let alert = $state<DockAlert | null>(null);
  /** Bumped by a new alert (not an update), so only a new one replays the entrance. */
  let alertSeq = $state(0);
  let leaving = $state(false);
  // The focus view (above) steps aside for an alert.
  const showFocus = $derived(!!focus && focusShown({ on: focusOn, active: true, peek, edit, alert: !!alert }));
  let alertTimers: ReturnType<typeof setTimeout>[] = [];
  const alertCfg = $derived({ ...ALERT_DEFAULTS, ...(store.config?.dockAlerts ?? {}) });
  const view = $derived(alert ? viewOf(alert) : null);
  // The prayer alert says it for a few seconds (holdMs), with Stop adhan while the adzan
  // plays; the adzan goes on after it, and the prayer widget and its panel can stop it too.

  // The charger's switch is the older `chargerFlash`, checked here rather than in Rust.
  const enabled = (k: AlertKind) => (k === 'charger' ? store.config?.chargerFlash !== false : alertCfg[k] !== false);
  const current = () => (alert && !leaving ? alert.kind : null);
  function clearAlertTimers() { alertTimers.forEach(clearTimeout); alertTimers = []; }
  function endAlert() {
    clearAlertTimers();
    alert = null;
    leaving = false;
    ipc.dockAlertEnd().catch(() => {});
  }
  /** Stay readable for `ms`, then leave -- but never from under the pointer: while the mouse
   *  is on the alert (also one that appeared under a still mouse) it waits, and goes a
   *  moment after the mouse moves off. */
  let flashEl = $state<HTMLElement>();
  function holdFor(ms: number) {
    clearAlertTimers();
    alertTimers = [setTimeout(function go() {
      if (flashEl?.matches(':hover')) { alertTimers = [setTimeout(go, 400)]; return; }
      leaving = true;
      alertTimers = [setTimeout(endAlert, FLASH.exit)];
    }, ms)];
  }
  async function showAlert(a: DockAlert) {
    // Mid-drag it would sit under the floating copy and look broken; the dock is busy anyway.
    if ((!enabled(a.kind) && !a.test) || dragId || arrive(current(), a.kind) === 'drop') return;
    const hold = holdMs(a.kind, alertCfg.length);
    // Rust refuses over a full-screen app, and slides an auto-hidden dock out for the alert.
    if (!(await ipc.dockAlertBegin(a.kind, FLASH.enter + hold + FLASH.exit).catch(() => true))) return;
    const how = arrive(current(), a.kind); // again: another alert may have come in meanwhile
    if (how === 'drop') return;
    alert = a;
    leaving = false;
    if (how === 'show') alertSeq++;
    holdFor((how === 'show' ? FLASH.enter : 0) + hold);
  }
  // Water: "+ cup" logs it, the fill rises to the new total, then the alert goes; "Later"
  // snoozes reminders for 15 minutes. A Settings test only pretends.
  async function drink() {
    if (alert?.kind !== 'water') return;
    const a = alert;
    const total = a.test ? a.total + a.cup : (await ipc.waterAdd(a.cup).catch(() => null))?.total;
    if (total === undefined || alert !== a) return;
    alert = { ...a, total, reached: total >= a.goal };
    holdFor(1400);
  }
  function later() {
    if (alert?.kind === 'water' && !alert.test) ipc.waterSnooze(15).catch(() => {});
    dismissAlert();
  }

  /** A button on an alert: do it, then the alert has done its job. */
  function act(f: () => Promise<unknown>) { f().catch(() => {}); dismissAlert(); }

  // Hovering holds it; moving away gives it a short moment more. A click dismisses it.
  const pauseAlert = () => { if (alert && !leaving) clearAlertTimers(); };
  const resumeAlert = () => { if (alert && !leaving) holdFor(1200); };
  function dismissAlert() {
    if (!alert || leaving) return;
    clearAlertTimers();
    leaving = true;
    alertTimers = [setTimeout(endAlert, FLASH.exit)];
  }

  // After an update: "What's new" once, from the main dock, when the dock has settled.
  onMount(() => {
    if (!isMainDock(win.label)) return;
    const t = setTimeout(async () => {
      const v = await ipc.whatsNewPending().catch(() => null);
      if (!v || !scroller) return;
      const r = (scroller.closest('.bar') as HTMLElement).getBoundingClientRect();
      const mid = new DOMRect(r.x + r.width / 2 - 20, r.y, 40, r.height);
      ipc.popupOpen(`__whatsnew:${v}`, mid, 420, 520, true).catch(() => {});
    }, 3000);
    return () => clearTimeout(t);
  });

  onMount(() => {
    const offs = [
      // Self-check: answer the core's ping through the same path the buttons use.
      events.healthPing((seq) => { ipc.healthPong(seq).catch(() => {}); }),
      events.editMode((on) => (edit = on)),
      win.onMoved(onMoved),
      // Alert sounds play here: the bar window is always alive.
      // After a screenshot / OCR snap: sound + a feedback card next to the Capture widget.
      events.captureDone((d) => {
        if (!isMainDock(win.label)) return; // one shutter and one card, from the main dock
        if (d.kind === 'shot') shutter(); else if (d.kind === 'ocr' || d.kind === 'color') tick();
        const el = document.querySelector('.capture-pill') ?? scroller;
        const h = d.kind === 'shot' ? 208 : d.kind === 'ocr' ? 168 : d.kind === 'color' ? 176 : 100;
        setTimeout(() => ipc.popupOpen('__feedback', el.getBoundingClientRect(), 300, h, false), 60);
      }),
      // A notification was clicked: scroll to the widget that sent it, pulse a soft glow,
      // and (if set) open its panel. Removed since: the Notification Center instead.
      events.barJump(({ source, open }) => {
        const t = jumpTarget(source, shown);
        if (t.kind === 'none') return;
        const el = scroller.querySelectorAll<HTMLElement>(':scope > .slot')[t.index];
        if (!el) return;
        el.scrollIntoView({ behavior: 'smooth', inline: 'center', block: 'center' });
        el.classList.remove('jump'); void el.offsetWidth; el.classList.add('jump');
        setTimeout(() => el.classList.remove('jump'), 2300);
        if (t.kind === 'widget' && !open) return;
        setTimeout(() => {
          const w = shown[t.index];
          if (t.kind === 'widget' && t.part) {
            const seg = el.querySelectorAll<HTMLElement>('.seg')[PARTS.filter((p) => (w.settings as Record<string, unknown> | null)?.[p.show] !== false).findIndex((p) => p.key === t.part)] ?? el;
            const p = PARTS.find((x) => x.key === t.part)!;
            ipc.popupOpen(partPanel(p.key, w.instance), seg.getBoundingClientRect(), p.width, p.height);
          } else {
            const m = widgets[w.id];
            if (m?.Popup) openPopup(w.instance, el, m.Popup.width, m.Popup.height);
          }
        }, 450); // after the scroll settles
      }),
      events.soundPlay((s) => (s.kind === 'azan' || s.kind === 'adhan' ? (setAdhanPlaying(true), playAdzan(s.kind === 'azan' ? s.src : adhanOf(s.voice).url, !s.short ? false : s.kind === 'azan' || adhanOf(s.voice).short, !!s.pauseMusic, { pause: ipc.mediaPausePlaying, resume: ipc.mediaResume }, s.volume ?? 1, () => setAdhanPlaying(false))) : s.kind === 'connect' || s.kind === 'disconnect' ? deviceTone(s.kind === 'connect', s.volume) : s.kind === 'plug' || s.kind === 'unplug' ? chargerTone(s.kind === 'plug', s.volume) : chime(s.volume))),
      events.chargerFlip((f) => showAlert({ kind: 'charger', ...f })),
      events.popupShow((i) => { panelOpen = i !== '__feedback'; }),
      events.popupHide(() => { panelOpen = false; }),
      events.dockAlert(showAlert),
    ];
    return () => {
      offs.forEach((p) => p.then((off) => off()));
      clearAlertTimers();
    };
  });

  // ---- Move: drag the bar background, snap to an edge on release ----
  let dragging = false;
  let dragTimer: ReturnType<typeof setTimeout>;

  let dragFrom = { x: 0, y: 0 };

  async function onBarDown(e: PointerEvent) {
    if (e.button !== 0 || edit || (e.target as HTMLElement).closest('.slot, .handle, .edit-tools')) return;
    dragging = true;
    const p = await win.innerPosition();
    dragFrom = { x: p.x, y: p.y };
    await ipc.barDrag(true);
    win.startDragging();
    dragTimer = setTimeout(finishDrag, 250); // also ends a plain click
  }

  function onMoved() {
    if (!dragging) return; // ignore moves we made ourselves (apply_bar)
    clearTimeout(dragTimer);
    dragTimer = setTimeout(finishDrag, 250);
  }

  async function finishDrag() {
    // A pause mid-drag is not a drop: wait until the button is released.
    if (await ipc.mouseDown()) { dragTimer = setTimeout(finishDrag, 150); return; }
    dragging = false;
    await ipc.barDrag(false);
    const [pos, size, scale, mons] = await Promise.all([win.innerPosition(), win.innerSize(), win.scaleFactor(), availableMonitors()]);
    if (pos.x === dragFrom.x && pos.y === dragFrom.y) return; // a click, not a drag
    const rects = mons.map((m) => ({ x: m.position.x, y: m.position.y, w: m.size.width, h: m.size.height }));
    const s = snap({ x: pos.x, y: pos.y, w: size.width, h: size.height }, rects, 48 * scale);
    await update((c) => {
      const b = c.profiles[c.activeProfile].bar;
      b.monitor = s.monitor;
      b.autoHide = s.hide; // dragged past the edge = hide there; dragged back in = stay visible
      if (s.edge) { b.edge = s.edge; b.floating = null; } else { b.floating = { x: pos.x, y: pos.y }; }
    });
  }

  // ---- Resize: drag either end to change the bar's length ----
  // Only the window resizes while dragging (bar_preview); config is saved once on release.
  function onHandleDown(e: PointerEvent, side: 1 | -1) {
    if (e.button !== 0) return;
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const start = vertical ? e.screenY : e.screenX;
    const startLen = bar.length;
    let span = 0;
    let length = startLen;
    Promise.all([availableMonitors(), primaryMonitor()]).then(([mons, primary]) => {
      const m = (bar.monitor !== null ? mons[bar.monitor] : primary) ?? mons[0];
      span = (vertical ? m.size.height : m.size.width) / m.scaleFactor;
    });
    const move = (ev: PointerEvent) => {
      if (!(ev.buttons & 1)) return done();
      if (!span) return;
      // The bar is centered, so it grows on both sides: double the pointer delta.
      const d = ((vertical ? ev.screenY : ev.screenX) - start) * side * 2;
      length = Math.min(1, Math.max(0.2, startLen + d / span));
      ipc.barPreview(length);
    };
    const done = () => {
      el.removeEventListener('pointermove', move);
      el.removeEventListener('lostpointercapture', done);
      if (length !== startLen) update((c) => { c.profiles[c.activeProfile].bar.length = length; });
    };
    el.addEventListener('pointermove', move);
    el.addEventListener('lostpointercapture', done); // fires on pointerup and on cancel
  }

  // ---- Scroll: wheel scrolls overflowing widgets with momentum ----
  $effect(() => {
    let v = 0, raf = 0;
    const step = () => {
      if (vertical) scroller.scrollTop += v; else scroller.scrollLeft += v;
      v *= 0.86;
      raf = Math.abs(v) > 0.3 ? requestAnimationFrame(step) : 0;
    };
    const onWheel = (e: WheelEvent) => {
      if (e.defaultPrevented) return;
      e.preventDefault();
      v += (Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY) * 0.3;
      if (!raf) raf = requestAnimationFrame(step);
    };
    scroller.addEventListener('wheel', onWheel, { passive: false });
    return () => { scroller.removeEventListener('wheel', onWheel); cancelAnimationFrame(raf); };
  });

  // ---- More widgets than fit: fade the cut-off end(s) and show "▾ 6" there (click scrolls) ----
  let more = $state({ before: 0, after: 0 });
  let measureRaf = 0;
  function measure() {
    cancelAnimationFrame(measureRaf);
    measureRaf = requestAnimationFrame(() => {
      if (!scroller) return;
      const slots = [...scroller.querySelectorAll<HTMLElement>(':scope > .slot')].map((s) => s.getBoundingClientRect());
      const next = hiddenCounts(slots, scroller.getBoundingClientRect(), vertical);
      if (next.before !== more.before || next.after !== more.after) more = next;
    });
  }
  $effect(() => {
    shown.length; vertical; // re-measure when widgets or the edge change
    measure();
    const ro = new ResizeObserver(() => { measure(); revealSoon(); });
    ro.observe(scroller);
    for (const s of scroller.children) ro.observe(s);
    scroller.addEventListener('scroll', measure, { passive: true });
    return () => { ro.disconnect(); scroller.removeEventListener('scroll', measure); cancelAnimationFrame(measureRaf); clearTimeout(revealTimer); };
  });

  // ---- Dragging past the visible part of the dock -- a widget being moved, or an app icon
  // on its way to a folder: holding it near either end scrolls the dock that way (faster the
  // closer to the end), and the drop spot keeps up with the scroll, so a widget can go all
  // the way to either end in one move. ----
  $effect(() => {
    let v = 0, raf = 0;
    let at: { x: number; y: number } | null = null;
    const dragging = () => !!document.documentElement.dataset.appDrag || dragId !== null;
    const step = () => {
      if (!dragging() || !v) { raf = 0; return; }
      const before = vertical ? scroller.scrollTop : scroller.scrollLeft;
      if (vertical) scroller.scrollTop += v; else scroller.scrollLeft += v;
      const moved = (vertical ? scroller.scrollTop : scroller.scrollLeft) !== before;
      raf = moved ? requestAnimationFrame(step) : 0; // at the very end: stop
      // The pointer is still, but what's under it moved: let the drag work out its spot again.
      if (moved && at) dispatchEvent(new PointerEvent('pointermove', { clientX: at.x, clientY: at.y, buttons: 1 }));
    };
    const move = (e: PointerEvent) => {
      at = { x: e.clientX, y: e.clientY };
      if (!dragging()) { v = 0; return; }
      const r = scroller.getBoundingClientRect();
      const [p, a, b] = vertical ? [e.clientY, r.top, r.bottom] : [e.clientX, r.left, r.right];
      const zone = 56;
      v = p < a + zone ? -Math.ceil((a + zone - p) / 4) : p > b - zone ? Math.ceil((p - (b - zone)) / 4) : 0;
      if (v && !raf) raf = requestAnimationFrame(step);
    };
    addEventListener('pointermove', move);
    return () => { removeEventListener('pointermove', move); cancelAnimationFrame(raf); };
  });

  // ---- A widget that opens up under the pointer (the File Shelf, the launcher's +N, tray
  // icons) and now runs past the visible part of the dock: scroll so all of it shows, once it
  // has finished growing. Only the hovered one, so nothing else moves the dock by itself. ----
  let revealTimer: ReturnType<typeof setTimeout>;
  function revealSoon() { clearTimeout(revealTimer); revealTimer = setTimeout(revealHovered, 140); }
  function revealHovered() {
    if (dragId || edit) return;
    const slot = scroller.querySelector<HTMLElement>(':scope > .slot:hover');
    if (!slot) return;
    // 44 px: clear of the fade and the "› N" chip at the ends.
    const by = revealBy(slot.getBoundingClientRect(), scroller.getBoundingClientRect(), vertical, 44);
    if (by) scroller.scrollBy({ [vertical ? 'top' : 'left']: by, behavior: 'smooth' });
  }
  function page(dir: -1 | 1) {
    const by = (vertical ? scroller.clientHeight : scroller.clientWidth) * 0.8 * dir;
    scroller.scrollBy({ [vertical ? 'top' : 'left']: by, behavior: 'smooth' });
  }
  const Back = $derived(vertical ? ChevronUp : ChevronLeft);
  const Fwd = $derived(vertical ? ChevronDown : ChevronRight);

  // ---- Widgets ----
  function onSlotClick(w: WidgetInst, el: HTMLElement) {
    if (swallowClick) { swallowClick = false; return; } // the click that ended a long-press
    const m = widgets[w.id];
    // A widget can ask for a wider panel right now (music with lyrics beside the player).
    const want = Number(el.querySelector<HTMLElement>('[data-popup-width]')?.dataset.popupWidth) || 0;
    if (!edit && m.Popup) openPopup(w.instance, el, want || m.Popup.width, m.Popup.height);
  }

  async function onContext(e: MouseEvent) {
    e.preventDefault();
    await showMenu([
      { text: edit ? 'Done editing' : 'Edit widgets', action: () => ipc.setEditMode(!edit) },
      // Dock placement right here, no Settings window needed.
      { text: 'Position', items: (['top', 'bottom', 'left', 'right'] as const).map((e) => ({
        text: e[0].toUpperCase() + e.slice(1), checked: !bar.floating && bar.edge === e,
        action: () => setBar((b) => { b.edge = e; b.floating = null; }),
      })) },
      { text: 'Auto-hide', checked: autoHide, action: () => setBar((b) => { b.autoHide = !autoHide; b.floating = null; }) },
      '-',
      { text: 'Settings…', action: () => ipc.openSettings() },
      '-',
      { text: 'Task Manager', action: () => ipc.appLaunch('taskmgr.exe') },
      { text: 'Show the desktop', action: () => ipc.showDesktop() },
      '-',
      { text: 'Close GlassBar', action: () => ipc.appQuit() },
    ]);
  }

  const removeWidget = (instance: string) =>
    update((c) => { const p = c.profiles[c.activeProfile]; p.widgets = p.widgets.filter((w) => w.instance !== instance); });

  const cycleSize = (w: WidgetInst) => {
    const sizes = widgets[w.id].sizes;
    const next: WidgetSize = sizes[(sizes.indexOf(w.size) + 1) % sizes.length];
    update((c) => { const t = c.profiles[c.activeProfile].widgets.find((x) => x.instance === w.instance); if (t) t.size = next; });
  };

  // The click that ends a pick-up must not open the widget's popup.
  let swallowClick = false;

  // ---- Free drag ----
  // Pick a widget up: in edit mode hold ~250 ms or start moving; otherwise press and hold
  // ~500 ms without moving (a quick move stays the widget's own tap or scroll). It follows
  // the pointer as a floating copy; a placeholder marks where it will land and the others
  // slide aside (FLIP via animate:flip). Drop anywhere; Esc puts it back. Saved on drop.
  let dragId = $state<string | null>(null);
  let ghost = $state<{ x: number; y: number; w: number; h: number } | null>(null);

  // Press-and-hold anywhere on a widget picks it up, even on its buttons (Downloads' file,
  // Start, the volume): the slot listens in the capture phase, before a button stops the
  // press. Only parts with their own drag (App Launcher icons, Capture buttons, shelved files:
  // `data-own-drag`) keep theirs outside edit mode.
  function onSlotDown(e: PointerEvent, instance: string) {
    swallowClick = false;
    if (e.button !== 0 || (e.target as HTMLElement).closest('.badge')) return;
    const editing = edit;
    const el = e.currentTarget as HTMLElement;
    // Track the pointer on the window, not with pointer capture on the slot: reordering
    // moves the slot's DOM node, which silently drops capture and ended the drag after
    // the first swap.
    const start = { x: e.clientX, y: e.clientY };
    const before = placed.map((w) => w.instance).join();
    let grab = { dx: 0, dy: 0 };
    let last = e;
    let raf = 0;
    let picked = false;

    const pick = () => {
      if (picked) return;
      picked = true;
      swallowClick = true;
      const r = el.getBoundingClientRect();
      grab = { dx: start.x - r.left, dy: start.y - r.top };
      ghost = { x: r.left, y: r.top, w: r.width, h: r.height };
      dragId = instance;
      order = placed.map((w) => w.instance);
    };
    const pickTimer = setTimeout(pick, editing ? 250 : 500);

    // Where the dragged widget goes is decided by the pointer against the other widgets'
    // *layout* centres (offsetLeft/Top: the scroller is their offset parent; only the dock's
    // own slots, not a widget's inner elements that happen to be called "slot").
    // On-screen boxes would include the slide (flip) animation in progress, and the target
    // flipped back and forth while the widgets moved. Near either end the dock scrolls.
    const EDGE = 48;
    const frame = () => {
      raf = 0;
      if (!ghost || !order) return;
      ghost = { ...ghost, x: vertical ? ghost.x : last.clientX - grab.dx, y: vertical ? last.clientY - grab.dy : ghost.y };
      const box = scroller.getBoundingClientRect();
      const p = vertical ? last.clientY : last.clientX;
      const [lo, hi] = vertical ? [box.top, box.bottom] : [box.left, box.right];
      const speed = p < lo + EDGE ? -Math.ceil((lo + EDGE - p) / 4) : p > hi - EDGE ? Math.ceil((p - (hi - EDGE)) / 4) : 0;
      if (speed) { if (vertical) scroller.scrollTop += speed; else scroller.scrollLeft += speed; }
      // The pointer decides (it goes where you point), in the scroller's layout coordinates.
      const centre = p - (vertical ? box.top : box.left) + (vertical ? scroller.scrollTop : scroller.scrollLeft);
      const others = [...scroller.querySelectorAll<HTMLElement>(':scope > .slot:not(.placeholder)')]
        .map((s) => (vertical ? s.offsetTop + s.offsetHeight / 2 : s.offsetLeft + s.offsetWidth / 2));
      const to = insertIndex(others, centre);
      const rest = order.filter((id) => id !== instance);
      const next = [...rest.slice(0, to), instance, ...rest.slice(to)];
      if (next.join() !== order.join()) order = next;
      if (speed) raf = requestAnimationFrame(frame); // keep scrolling while held at an edge
    };
    const move = (ev: PointerEvent) => {
      last = ev;
      if (!picked && Math.hypot(ev.clientX - start.x, ev.clientY - start.y) > 6) {
        clearTimeout(pickTimer);
        if (editing) pick(); else return reset(); // moved before the hold: not a pick-up
      }
      if (picked && !raf) raf = requestAnimationFrame(frame);
    };
    // Once picked up, the widget's own drag (a shelved file, an icon image) must not start:
    // the browser's drag cancels the pointer and the widget dropped wherever it was.
    const noNativeDrag = (d: DragEvent) => { if (picked) { d.preventDefault(); d.stopPropagation(); } };
    const reset = () => {
      clearTimeout(pickTimer);
      cancelAnimationFrame(raf);
      removeEventListener('pointermove', move);
      removeEventListener('pointerup', up);
      removeEventListener('pointercancel', cancel);
      removeEventListener('keydown', onKey);
      removeEventListener('dragstart', noNativeDrag, true);
      dragId = null;
      ghost = null;
    };
    const onKey = (k: KeyboardEvent) => {
      if (k.key !== 'Escape') return;
      order = null; // back to where it was
      reset();
    };
    // Windows took the pointer away (a system gesture): put the widget back.
    const cancel = () => { order = null; reset(); };
    const up = async () => {
      const final = order;
      reset();
      try {
        if (picked && final && final.join() !== before) {
          await update((c) => {
            const p = c.profiles[c.activeProfile];
            p.widgets = final.map((id) => p.widgets.find((w) => w.instance === id)!).filter(Boolean)
              .concat(p.widgets.filter((w) => !final.includes(w.instance)));
          });
        }
      } finally {
        order = null;
      }
    };
    addEventListener('dragstart', noNativeDrag, true);
    addEventListener('pointermove', move);
    addEventListener('pointerup', up);
    addEventListener('pointercancel', cancel);
    addEventListener('keydown', onKey);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="surface bar dock" class:vertical class:edit class:flashing={!!alert} class:focusing={showFocus} onpointerdown={onBarDown} oncontextmenu={onContext}
     oncontextmenucapture={() => ipc.popupHoverHide()}
     onpointerenter={onEnter} onpointermove={focusMove} onpointerleave={focusLeave} data-size={bar.size} data-side={bar.sideWidth ?? 'regular'}>
  <div class="handle start" onpointerdown={(e) => onHandleDown(e, -1)}></div>

  <div class="scrollbox">
  <div class="scroller" bind:this={scroller} class:fade-before={more.before > 0} class:fade-after={more.after > 0}>
    {#each shown as w (w.instance)}
      {@const m = widgets[w.id]}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        class="slot"
        class:placeholder={dragId === w.instance}
        style:width={dragId === w.instance && ghost && !vertical ? `${ghost.w}px` : null}
        style:height={dragId === w.instance && ghost && vertical ? `${ghost.h}px` : null}
        class:dimmed={focusDim && !edit && w.id !== 'productivity'}
        role="button"
        tabindex="-1"
        animate:flip={{ duration: 280 }}
        onpointerdowncapture={(e) => { if (edit || !(e.target as Element).closest('[data-own-drag], input, textarea, select')) onSlotDown(e, w.instance); }}
        onclickcapture={(e) => { if (swallowClick) { swallowClick = false; e.stopPropagation(); e.preventDefault(); } }}
        onclick={(e) => onSlotClick(w, e.currentTarget as HTMLElement)}
      >
        <m.Pill instance={w.instance} size={vertical ? 'small' : w.size} settings={settingsOf(m, w)} {vertical} />
        {#if edit}
          <button class="badge remove" title="Remove" onclick={() => removeWidget(w.instance)}><X size={11} strokeWidth={3} /></button>
          {#if m.sizes.length > 1}
            <button class="badge resize" title="Size: {w.size}" onclick={() => cycleSize(w)}><Maximize2 size={10} strokeWidth={3} /></button>
          {/if}
        {/if}
      </div>
    {/each}
    {#if placed.length === 0}
      <span class="dim empty">Right-click → Edit widgets</span>
    {/if}
  </div>
  {#if more.before}
    <button class="more before" title="{more.before} more" aria-label="Show {more.before} more widgets" onpointerdown={(e) => e.stopPropagation()} onclick={() => page(-1)}>
      <Back size={12} strokeWidth={2.6} /><span>{more.before}</span>
    </button>
  {/if}
  {#if more.after}
    <button class="more after" title="{more.after} more" aria-label="Show {more.after} more widgets" onpointerdown={(e) => e.stopPropagation()} onclick={() => page(1)}>
      <Fwd size={12} strokeWidth={2.6} /><span>{more.after}</span>
    </button>
  {/if}
  </div>

  {#if showFocus && focus}
    <FocusDock f={focus} {vertical} />
  {/if}

  {#if alert && view}
    {#key alertSeq}
    <!-- Hovering holds it, a click dismisses it; buttons inside act without dismissing. -->
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
    <div class="flash" class:out={leaving} bind:this={flashEl} role="status" aria-live="polite"
         style:--c={view.tone} style:--p={view.fill ?? 0}
         style:--f-enter="{FLASH.enter}ms" style:--f-fill="{FLASH.fill}ms" style:--f-exit="{FLASH.exit}ms"
         onpointerenter={pauseAlert} onpointerleave={resumeAlert} onpointerdown={(e) => e.stopPropagation()} onclick={dismissAlert}>
      {#if view.fill !== null}<div class="flash-fill"></div>{/if}
      <div class="flash-body">
        {#if view.icon === 'plug'}<PlugZap size={16} />{:else if view.icon === 'unplug'}<Unplug size={16} />{:else if view.icon === 'volume'}<Volume2 size={16} />{:else if view.icon === 'mute'}<VolumeX size={16} />{:else if view.icon === 'sun'}<Sun size={16} />{:else if view.icon === 'water'}<Droplet size={16} />{:else if view.icon === 'break'}<Coffee size={16} />{:else if view.icon === 'focus'}<Target size={16} />{:else if view.icon === 'timer'}<TimerIcon size={16} />{:else}<MoonStar size={16} />{/if}
        <strong>{view.value}</strong>
        <span class="flash-label">{view.label}</span>
        {#if alert.kind === 'water' && !alert.reached}
          <button class="flash-act" onclick={(e) => { e.stopPropagation(); drink(); }} aria-label="Log a drink of {fmtWater(alert.cup, alert.unit)}">
            <Plus size={11} /><span>{fmtWater(alert.cup, alert.unit)}</span>
          </button>
          <button class="flash-act quiet" onclick={(e) => { e.stopPropagation(); later(); }} aria-label="Remind me in 15 minutes"><span>Later</span></button>
        {/if}
        {#if alert.kind === 'pomodoro'}
          {#if alert.auto}
            <button class="flash-act" onclick={(e) => { e.stopPropagation(); act(() => ipc.pomoPause()); }}><Pause size={11} /><span>Pause</span></button>
          {:else}
            <button class="flash-act" onclick={(e) => { e.stopPropagation(); act(() => ipc.pomoStart()); }}><Play size={11} /><span>{alert.done === 'focus' ? 'Start break' : 'Start focus'}</span></button>
          {/if}
          <button class="flash-act quiet" onclick={(e) => { e.stopPropagation(); act(() => ipc.pomoSkip()); }}><SkipForward size={11} /><span>Skip</span></button>
        {/if}
        {#if alert.kind === 'timer' && alert.id !== null}
          {@const id = alert.id}
          <button class="flash-act" onclick={(e) => { e.stopPropagation(); act(() => ipc.timerSnooze(id, 60_000)); }}><Plus size={11} /><span>1 min</span></button>
          <button class="flash-act quiet" onclick={(e) => { e.stopPropagation(); act(() => ipc.timerSnooze(id, 300_000)); }}><span>+5 min</span></button>
        {/if}
        {#if alert.kind === 'prayer' && adhan.playing}
          <button class="flash-act" onclick={(e) => { e.stopPropagation(); stopAdhan(); }} aria-label="Stop the adhan">
            <Square size={10} fill="currentColor" /><span>Stop adhan</span>
          </button>
        {/if}
      </div>
    </div>
    {/key}
  {/if}

  {#if ghost && dragId}
    {@const w = placed.find((x) => x.instance === dragId)}
    {#if w}
      {@const m = widgets[w.id]}
      <div class="ghost" style:width="{ghost.w}px" style:height="{ghost.h}px" style:transform="translate({ghost.x}px, {ghost.y}px) scale(1.06)">
        <m.Pill instance={w.instance} size={vertical ? 'small' : w.size} settings={settingsOf(m, w)} {vertical} />
      </div>
    {/if}
  {/if}

  {#if edit}
    <div class="edit-tools">
      <button class="icon-btn" title="Add widget" onclick={(e) => openPopup(GALLERY, e.currentTarget, 340, 460)}><Plus size={16} /></button>
      <button class="icon-btn done" title="Done" onclick={() => ipc.setEditMode(false)}><Check size={16} /></button>
    </div>
  {/if}

  <div class="handle end" onpointerdown={(e) => onHandleDown(e, 1)}></div>
</div>

<style>
  .bar {
    display: flex;
    align-items: stretch;
    gap: var(--gap);
    padding: calc(5px * var(--sp));
    position: relative;
  }
  .bar.vertical { flex-direction: column; }
  .bar[data-size='compact'] { padding: calc(4px * var(--sp)); }

  .scroller {
    flex: 1;
    display: flex;
    align-items: stretch;
    /* Centred with equal space both sides; 'safe' falls back to start when the
       widgets overflow, so the first one stays reachable by scrolling. */
    justify-content: safe center;
    gap: var(--gap);
    overflow: hidden;
    min-width: 0;
    min-height: 0;
  }
  .vertical .scroller { flex-direction: column; }
  /* The widgets' offset parent, so drag-to-reorder can read their layout positions. */
  .scroller { position: relative; }
  .scrollbox { flex: 1; position: relative; display: flex; min-width: 0; min-height: 0; }
  /* Cut-off ends fade out, so it's clear the row/column goes on. */
  .scroller { --fb: 0px; --fa: 0px; }
  .scroller.fade-before { --fb: 26px; }
  .scroller.fade-after { --fa: 26px; }
  .scroller.fade-before, .scroller.fade-after {
    -webkit-mask-image: linear-gradient(to right, transparent, #000 var(--fb), #000 calc(100% - var(--fa)), transparent);
    mask-image: linear-gradient(to right, transparent, #000 var(--fb), #000 calc(100% - var(--fa)), transparent);
  }
  .vertical .scroller.fade-before, .vertical .scroller.fade-after {
    -webkit-mask-image: linear-gradient(to bottom, transparent, #000 var(--fb), #000 calc(100% - var(--fa)), transparent);
    mask-image: linear-gradient(to bottom, transparent, #000 var(--fb), #000 calc(100% - var(--fa)), transparent);
  }
  /* Paging chip over the widgets: the dock's own material (its tint and opacity, a pill
     layer on top, the widgets under it blurred), so it matches every theme and style. */
  .more {
    --fill: var(--pill);
    position: absolute; z-index: 3; display: flex; align-items: center; gap: 1px;
    padding: 2px 6px 2px 4px; border-radius: var(--r-chip); font-size: 10px; font-weight: 600; font-variant-numeric: tabular-nums;
    background: linear-gradient(var(--fill), var(--fill)), rgba(var(--tint), max(var(--tint-alpha), 0.6));
    backdrop-filter: blur(var(--blur)) saturate(1.4);
    border: 1px solid var(--line); box-shadow: inset 0 1px 0 var(--line), var(--shadow); color: var(--fg);
    transition: background 0.2s var(--ease), transform 0.35s var(--spring);
  }
  .more:hover { --fill: var(--pill-hover); }
  /* Shown only while the pointer is on the dock. */
  .more { transition: background 0.2s var(--ease), transform 0.35s var(--spring), opacity 0.2s var(--ease); }
  .bar:not(:hover) .more { opacity: 0; pointer-events: none; }
  .more:active { transform: scale(0.95); }
  /* Transparency effects off: solid, like the dock. */
  :global(:root[data-solid='true']) .more { background: linear-gradient(var(--fill), var(--fill)), rgb(var(--tint)); backdrop-filter: none; }
  /* Liquid Glass: glass body, bright rim, specular top edge; the sheen on hover. */
  :global(:root[data-preset='liquid']) .more {
    background: linear-gradient(180deg, var(--glass-top), var(--glass-bot)), rgba(var(--tint), max(var(--tint-alpha), 0.35));
    border-color: var(--glass-rim);
    box-shadow: inset 0 1px 0 var(--glass-spec), inset 0 0 0 0.5px rgba(255, 255, 255, 0.15), var(--glass-shadow);
  }
  :global(:root[data-preset='liquid']) .more:hover {
    background: radial-gradient(120% 60% at 30% 0%, var(--sheen), transparent 60%), linear-gradient(180deg, var(--glass-top), var(--glass-bot)), rgba(var(--tint), max(var(--tint-alpha), 0.35));
  }
  :global(:root:is([data-style='gnome'], [data-style='windows'])) .more { box-shadow: var(--shadow); }
  .more.before { left: 0; top: 50%; translate: 0 -50%; }
  .more.after { right: 0; top: 50%; translate: 0 -50%; }
  .vertical .more.before { left: 50%; top: 0; translate: -50% 0; }
  .vertical .more.after { left: 50%; top: auto; bottom: 0; translate: -50% 0; }

  /* Jumped to from a notification: a soft accent ring pulses 3 times (~2 s), then fades. */
  .slot:global(.jump) > :global(*) { animation: jumpglow 0.7s ease-in-out 3; border-radius: var(--radius-pill); }
  @keyframes jumpglow { 50% { box-shadow: 0 0 0 2px var(--accent), 0 0 16px color-mix(in srgb, var(--accent) 65%, transparent); } }
  /* where the dragged widget will land */
  .slot.dimmed:not(:hover) { opacity: 0.35; }
  .slot { transition: opacity 0.4s var(--ease); }
  .slot.placeholder { animation: none !important; border-radius: var(--radius-pill); outline: 2px dashed color-mix(in srgb, var(--accent) 70%, transparent); outline-offset: -2px; }
  .slot.placeholder > :global(*) { opacity: 0; }
  /* The gap keeps the size the widget had when picked up: a widget that is wider while
     hovered (Downloads) shrank once the pointer left it, shifting every target under it. */
  .slot.placeholder { overflow: hidden; flex: none; }
  .ghost {
    position: fixed; left: 0; top: 0; z-index: 10; pointer-events: none; display: flex;
    border-radius: var(--radius-pill); box-shadow: 0 10px 26px rgba(0, 0, 0, 0.35);
    transition: transform 40ms linear; will-change: transform;
  }
  /* Slot and left/right column rules are shared with the Settings preview: glass.css (.dock). */
  .edit .slot { animation: jiggle 0.28s ease-in-out infinite; }
  .edit .slot:nth-child(2n) { animation-delay: -0.14s; }

  /* ---- Dock alerts (the charger flash and its siblings) ----
     One thing across the whole dock for a moment (dockAlert.ts). Every rule
     declares its resting state and animates *from* somewhere else, so with motion turned off
     (which drops every animation) the flash still draws correctly — it appears instead of
     growing. Durations come from chargeFlash.ts, so the CSS and the timeouts can't drift. */
  .flashing .slot, .flashing .empty, .flashing .more { opacity: 0; pointer-events: none; }
  /* Focus on the dock: the widgets fade out under it (and back in on hover). */
  .focusing .slot, .focusing .empty, .focusing .more { opacity: 0; pointer-events: none; }
  .flash {
    position: absolute; inset: 0; z-index: 4; cursor: default;
    display: grid; place-items: center;
    border-radius: var(--r-window); overflow: hidden;
    background: color-mix(in srgb, var(--c) 8%, transparent);
    animation: flash-in var(--f-enter) var(--ease);
  }
  @keyframes flash-in { from { opacity: 0; transform: scale(0.96); } }
  .flash.out { animation: flash-out var(--f-exit) var(--ease) forwards; }
  @keyframes flash-out { to { opacity: 0; } }

  /* The level itself, sweeping in from the leading edge. Transform only, so it stays on the
     GPU compositor and never reflows the dock. */
  .flash-fill {
    position: absolute; inset: 0;
    transform-origin: left; transform: scaleX(var(--p));
    background: linear-gradient(90deg, color-mix(in srgb, var(--c) 45%, transparent), color-mix(in srgb, var(--c) 22%, transparent));
    animation: flash-sweep var(--f-fill) var(--ease) var(--f-enter) backwards;
    /* An update in place (volume-up again) slides the level instead of replaying the sweep. */
    transition: transform 0.3s var(--ease);
  }
  @keyframes flash-sweep { from { transform: scaleX(0); } }
  .vertical .flash-fill {
    transform-origin: bottom; transform: scaleY(var(--p));
    background: linear-gradient(0deg, color-mix(in srgb, var(--c) 45%, transparent), color-mix(in srgb, var(--c) 22%, transparent));
    animation-name: flash-sweep-v;
  }
  @keyframes flash-sweep-v { from { transform: scaleY(0); } }

  .flash-body { position: relative; display: flex; align-items: center; gap: 8px; min-width: 0; padding: 0 12px; color: var(--fg); }
  .flash-body :global(svg) { color: var(--c); flex: none; }
  .flash-body strong { font-size: 20px; font-weight: 600; line-height: 1; font-variant-numeric: tabular-nums; }
  .flash-label { font-size: 11px; color: var(--fg-dim); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  /* A button on the alert (Stop adhan): a quiet inset pill, so the value stays the hero. */
  .flash-act {
    display: inline-flex; align-items: center; gap: 6px; flex: none; height: 26px; padding: 0 10px; margin-left: 4px;
    border-radius: var(--r-chip); background: var(--bg-2, var(--pill)); color: var(--fg); font-size: 11px; font-weight: 500;
    border: 1px solid var(--line-soft); transition: background 0.2s var(--ease), transform 0.15s var(--ease);
    animation: flash-in var(--f-enter) var(--ease);
  }
  .flash-act:hover { background: var(--pill-hover); }
  .flash-act.quiet { background: transparent; border-color: transparent; color: var(--fg-dim); }
  .flash-act:active { transform: scale(0.96); }
  .flash-act :global(svg) { color: inherit; }
  .bar[data-size='compact'] .flash-body strong { font-size: 16px; }
  .bar[data-size='compact'] .flash-label { font-size: 10px; }
  /* A side dock is 56-96 px wide: icon and number only, no label to cut off. The number also
     has to shrink, or "100%" is clipped in a slim column (DESIGN_RULES section 8). */
  .vertical .flash-body { flex-direction: column; gap: 2px; padding: 0 4px; }
  .vertical .flash-label { display: none; }
  /* No room for words in a side column: the stop icon alone. */
  .vertical .flash-act { margin: 2px 0 0; padding: 0; width: 26px; justify-content: center; }
  .vertical .flash-act span { display: none; }
  .vertical .flash-act.quiet { display: none; }
  .bar.vertical .flash-body strong { font-size: 16px; }
  .bar.vertical[data-side='slim'] .flash-body strong { font-size: 13px; }

  .badge {
    position: absolute;
    width: 18px; height: 18px;
    display: grid; place-items: center;
    border-radius: 50%;
    color: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35);
    z-index: 2;
  }
  .remove { top: -3px; left: -3px; background: #ff453a; }
  .resize { bottom: -3px; right: -3px; background: var(--accent); color: var(--on-accent); }

  .edit-tools { display: flex; gap: 2px; align-items: center; }
  .vertical .edit-tools { flex-direction: column; }
  .done { background: var(--accent); color: var(--on-accent); }

  .handle { flex: none; width: 6px; cursor: ew-resize; border-radius: calc(3px * var(--rs)); }
  .vertical .handle { width: auto; height: 6px; cursor: ns-resize; }
  .handle:hover { background: var(--pill-hover); }

  .empty { align-self: center; padding: 0 12px; }
</style>
