// Liquid Glass: the sheen on the card under the pointer follows it (--mx, see glass.css).
// At most one update per frame, and only while the pointer moves, so an idle or hidden
// dock does no work. Off unless the theme is Liquid Glass with full motion.
import { getCurrentWindow } from '@tauri-apps/api/window';
import { events, ipc } from './ipc';
import { lensMap } from './lens';

const root = document.documentElement;
const reduced = matchMedia('(prefers-reduced-motion: reduce)');
let raf = 0;
let last: PointerEvent | null = null;

addEventListener('pointermove', (e) => {
  if (root.dataset.preset !== 'liquid' || root.dataset.motion !== 'full' || reduced.matches) return;
  last = e;
  raf ||= requestAnimationFrame(() => {
    raf = 0;
    const el = (last?.target as Element | null)?.closest?.<HTMLElement>('.pill, .card, .tile, .side, .glass');
    if (!el || !last) return;
    const r = el.getBoundingClientRect();
    el.style.setProperty('--mx', `${Math.round(((last.clientX - r.left) / Math.max(r.width, 1)) * 100)}%`);
  });
}, { passive: true });

// ---- What is behind the glass ----
// Rust pictures the screen behind this window (glass.rs). `use:glass` draws that picture
// inside the dock or the flyout, lightly blurred and bent at the rim by an SVG lens, so
// they read as thick glass over whatever is really there, not just a uniform blur.
// root[data-glass='on'] once a picture is up; until then the usual Windows blur shows.

type Pic = { url: string; x: number; y: number; w: number; h: number };
type Layer = { lg: HTMLElement; img: HTMLElement; panel: boolean; size: string };
let pic: Pic | null = null;
/** This window's client area on screen, physical px. */
let at = { x: 0, y: 0 };
const layers = new Map<HTMLElement, Layer>();
const win = getCurrentWindow();
const liquid = () => root.dataset.preset === 'liquid' && root.dataset.solid !== 'true';

let busy: Promise<void> | null = null;
let again = false;
async function load(fresh: boolean) {
  if (busy) { again = true; return busy; }
  busy = (async () => {
    try {
      if (!liquid() || !layers.size) return;
      const [buf, pos] = await Promise.all([ipc.glassBehind(fresh), win.innerPosition()]);
      if (buf.byteLength < 74 || !liquid()) return;
      const v = new DataView(buf);
      const [x, y, w, h] = [0, 4, 8, 12].map((o) => v.getInt32(o, true));
      const url = URL.createObjectURL(new Blob([buf.slice(20)], { type: 'image/bmp' }));
      // Decoded before it replaces the old one, so the glass never shows empty.
      const im = new Image();
      im.src = url;
      await im.decode().catch(() => {});
      const old = pic;
      pic = { url, x, y, w, h };
      at = { x: pos.x, y: pos.y };
      root.style.setProperty('--lg-lum', v.getFloat32(16, true).toFixed(3));
      root.dataset.glass = 'on'; // first: a hidden layer measures 0 x 0
      layers.forEach((l, el) => place(el, l));
      if (old) URL.revokeObjectURL(old.url);
    } catch { /* not ours to show: keep the Windows blur */ }
  })();
  await busy;
  busy = null;
  if (again) { again = false; return load(false); }
}

function place(el: HTMLElement, l: Layer) {
  if (!pic) return;
  const s = devicePixelRatio;
  const r = l.lg.getBoundingClientRect();
  l.img.style.backgroundImage = `url(${pic.url})`;
  l.img.style.backgroundSize = `${pic.w / s}px ${pic.h / s}px`;
  l.img.style.backgroundPosition = `${(pic.x - at.x) / s - r.left}px ${(pic.y - at.y) / s - r.top}px`;
  // A new lens only when the size, corners or blur changed (the map is per size).
  const w = Math.round(r.width), h = Math.round(r.height);
  const radius = parseFloat(getComputedStyle(el).borderTopLeftRadius) || 0;
  const blur = Math.round((parseFloat(getComputedStyle(root).getPropertyValue(l.panel ? '--glass-panel-blur' : '--glass-blur')) || 0) * 10) / 10;
  const key = `${w}x${h}r${radius}b${blur}`;
  if (w > 0 && h > 0 && key !== l.size) {
    l.size = key;
    l.img.style.filter = `url(#${lens(w, h, radius, blur)})`;
  }
}

/** Svelte action: the dock or (`panel`) the flyout surface gets a glass layer; its blur is
 *  Settings › Appearance › Dock / Panel glass blur. */
export function glass(el: HTMLElement, panel = false) {
  const lg = document.createElement('div');
  lg.className = 'lg';
  lg.innerHTML = '<div class="lg-img"></div><div class="lg-tint"></div><div class="lg-rim"></div>';
  el.prepend(lg);
  const l: Layer = { lg, img: lg.firstElementChild as HTMLElement, panel, size: '' };
  layers.set(el, l);
  const ro = new ResizeObserver(() => place(el, l));
  ro.observe(lg);
  if (pic) place(el, l); else void load(false);
  return { destroy() { ro.disconnect(); layers.delete(el); lg.remove(); } };
}

const made = new Map<string, string>();
let defs: SVGDefsElement | null = null;
function lens(w: number, h: number, r: number, blur: number): string {
  const key = `${w}x${h}r${r}b${blur}`;
  const have = made.get(key);
  if (have) return have;
  if (!defs) {
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('width', '0'); svg.setAttribute('height', '0');
    svg.style.position = 'absolute';
    defs = document.createElementNS('http://www.w3.org/2000/svg', 'defs');
    svg.append(defs);
    document.body.append(svg);
  }
  // A thin dock gets a thin rim, so most of it shows the picture straight.
  const band = Math.min(24, Math.min(w, h) * 0.2);
  const c = document.createElement('canvas');
  c.width = w; c.height = h;
  c.getContext('2d')!.putImageData(new ImageData(lensMap(w, h, r, band).data, w, h), 0, 0);
  const id = `lens${made.size}`;
  defs.insertAdjacentHTML('beforeend',
    `<filter id="${id}" x="0" y="0" width="${w}" height="${h}" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
      <feGaussianBlur in="SourceGraphic" stdDeviation="${blur}" edgeMode="duplicate" result="b"/>
      <feImage href="${c.toDataURL()}" x="0" y="0" width="${w}" height="${h}" preserveAspectRatio="none" result="m"/>
      <feDisplacementMap in="b" in2="m" scale="${Math.round(band * 2.6)}" xChannelSelector="R" yChannelSelector="G" result="d"/>
      <feColorMatrix in="d" type="saturate" values="1.45"/>
    </filter>`);
  made.set(key, id);
  return id;
}

// Rust has a new picture; the window moved; the theme changed to or from Liquid Glass.
events.glassBehind(() => void load(false)).catch(() => {});
win.onMoved(async () => {
  if (!pic) return;
  const p = await win.innerPosition();
  at = { x: p.x, y: p.y };
  layers.forEach((l, el) => place(el, l));
}).catch(() => {});
let was = liquid();
new MutationObserver(() => {
  const now = liquid();
  if (now !== was) {
    was = now;
    if (now) void load(true); else delete root.dataset.glass;
  }
  // Appearance changed (blur, corners): same picture, maybe a new lens.
  if (pic) layers.forEach((l, e) => place(e, l));
}).observe(root, { attributes: true, attributeFilter: ['data-preset', 'data-solid', 'style'] });
