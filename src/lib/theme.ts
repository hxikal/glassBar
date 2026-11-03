// Appearance settings -> data attributes + CSS variables on <html>, in every window.
// The tokens are defined in styles/glass.css; the presets and guards in appearance.ts.
import { events, ipc, type Theme } from './ipc';
import { accentInk, onAccent, resolve, rgb, windowRadius, type Behind } from './appearance';

const dark = matchMedia('(prefers-color-scheme: dark)');
let current: Theme | null = null;
let transparency = true;
dark.addEventListener('change', () => current && applyTheme(current));
/** Wallpaper brightness behind the dock (Rust checks it when the wallpaper changes). */
export const backdrop: { behind: Behind | null } = { behind: null };
const setBehind = (b: Behind | null) => { backdrop.behind = b; if (current) applyTheme(current); };
ipc.wallpaperBehind().then(setBehind).catch(() => {});
events.behindChanged(setBehind).catch(() => {});

export function applyTheme(t: Theme) {
  current = t;
  const r = resolve(t, { transparency, prefersDark: dark.matches, behind: backdrop.behind });
  const root = document.documentElement;
  const d = root.dataset;
  d.theme = r.dark ? 'dark' : 'light';
  // Native blur draws nothing while Windows' Transparency effects are off: draw the tint here then.
  d.native = String(r.blurred);
  // Without transparency effects Windows paints see-through pixels black: draw a solid surface.
  d.solid = String(!transparency);
  d.style = t.style;
  d.preset = t.preset;
  d.contrast = t.contrast;
  d.motion = t.motion;
  ipc.transparencyEffects().then((on) => { if (on !== transparency) { transparency = on; if (current) applyTheme(current); } }).catch(() => {});
  const set = (k: string, v: string | null) => (v === null ? root.style.removeProperty(k) : root.style.setProperty(k, v));
  set('--accent', t.accent);
  set('--on-accent', onAccent(t.accent));
  set('--accent-ink', accentInk(t.accent));
  set('--blur', `${t.blur}px`);
  set('--glass-blur', String(t.glassBlur ?? 2));
  set('--glass-panel-blur', String(t.glassPanelBlur ?? 8));
  // The dock window shows the dock opacity; panels, banners and previews the panel one.
  set('--tint-alpha', String(d.window === 'bar' ? r.dock : r.panel));
  set('--card-k', String(r.card));
  set('--tint', t.bg ? rgb(t.bg) : null);
  set('--rs', String(t.radius / 16));
  set('--r-window', `${windowRadius(t.radius, r.blurred)}px`);
  set('--shadow-k', String(t.shadow));
  set('--border-k', String(t.border));
  set('--icon-k', String(t.iconScale));
  set('--sp', String(t.spacing));
}
