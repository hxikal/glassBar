// Appearance presets and the one place settings become CSS tokens (see theme.ts).
// Two independent axes: a theme (colours + material) and a design style (shape).
import type { Theme } from './ipc';

export type ThemePreset = Theme['preset'];
export type Style = Theme['style'];

/** Fields owned by the theme axis and by the style axis. Accent and saved presets are neither. */
type ThemeFields = Pick<Theme, 'mode' | 'nativeBlur' | 'material' | 'opacity' | 'panelOpacity' | 'cardOpacity' | 'bg' | 'contrast' | 'blur' | 'glassBlur' | 'glassPanelBlur'>;
type StyleFields = Pick<Theme, 'radius' | 'shadow' | 'border' | 'iconScale' | 'spacing' | 'motion'>;

const base: ThemeFields = { mode: 'auto', nativeBlur: true, material: 'blur', opacity: 0, panelOpacity: 0, cardOpacity: 1, bg: null, contrast: 'normal', blur: 24, glassBlur: 2, glassPanelBlur: 8 };

/** `style`: a design style the theme brings along (Liquid Glass needs big soft corners). */
export const THEMES: Record<ThemePreset, { label: string; hint: string; set: ThemeFields; style?: Style }> = {
  system: { label: 'System', hint: 'Follows Windows light or dark mode', set: base },
  light: { label: 'Light', hint: 'Bright surfaces, dark text', set: { ...base, mode: 'light', opacity: 0.45, panelOpacity: 0.6 } },
  dark: { label: 'Dark', hint: 'Dark surfaces, light text', set: { ...base, mode: 'dark', opacity: 0.45, panelOpacity: 0.6 } },
  glass: { label: 'Glass', hint: 'See-through, blurred, layered', set: { ...base, cardOpacity: 0.55 } },
  // Thin dock, regular cards, thick flyouts (more tint so panel text stays readable).
  liquid: { label: 'Liquid Glass', hint: 'Apple-style glass with a soft sheen', set: { ...base, opacity: 0.06, panelOpacity: 0.35, blur: 20 }, style: 'mac' },
  custom: { label: 'Custom', hint: 'Your own colours', set: { ...base, mode: 'dark', bg: '#1e2433', opacity: 0.6, panelOpacity: 0.7 } },
};

export const STYLES: Record<Style, { label: string; hint: string; set: StyleFields }> = {
  glassbar: { label: 'GlassBar', hint: 'The original look', set: { radius: 16, shadow: 1, border: 1, iconScale: 1, spacing: 1, motion: 'full' } },
  mac: { label: 'macOS-inspired', hint: 'Rounder, airier, soft shadows', set: { radius: 20, shadow: 1.4, border: 0.7, iconScale: 1.05, spacing: 1.2, motion: 'full' } },
  windows: { label: 'Windows-inspired', hint: 'Fluent: small corners, calm motion', set: { radius: 8, shadow: 0.7, border: 1, iconScale: 1, spacing: 0.9, motion: 'reduced' } },
  gnome: { label: 'GNOME-inspired', hint: 'Flat, rounded, generous spacing', set: { radius: 12, shadow: 0.5, border: 0.4, iconScale: 1, spacing: 1.15, motion: 'full' } },
  kde: { label: 'KDE Plasma-inspired', hint: 'Crisp edges, compact panel', set: { radius: 6, shadow: 0.6, border: 1.3, iconScale: 0.95, spacing: 0.85, motion: 'full' } },
  cinnamon: { label: 'Cinnamon-inspired', hint: 'Classic square-ish panel', set: { radius: 3, shadow: 0.4, border: 1.2, iconScale: 0.95, spacing: 0.8, motion: 'reduced' } },
};

/** Every field a preset resets (and a saved preset stores). */
export const PRESET_KEYS = [...Object.keys(base), ...Object.keys(STYLES.glassbar.set), 'accent', 'preset', 'style'] as (keyof Theme)[];

/** The values of the chosen theme + style, i.e. what "Reset to preset" goes back to. */
export const presetDefaults = (t: Pick<Theme, 'preset' | 'style'>): ThemeFields & StyleFields =>
  ({ ...THEMES[t.preset].set, ...STYLES[t.style].set });

/** Fields that differ from the chosen preset (shown with a revert button). */
export const changed = (t: Theme, key: keyof (ThemeFields & StyleFields)) => presetDefaults(t)[key] !== t[key];

/** Snapshot for a saved preset, and applying one back. */
export const snapshot = (t: Theme) => Object.fromEntries(PRESET_KEYS.map((k) => [k, t[k]]));
export function applySnapshot(t: Theme, s: Record<string, unknown>) {
  for (const k of PRESET_KEYS) if (k in s) (t as unknown as Record<string, unknown>)[k] = s[k];
}

/** Relative luminance of #rrggbb, 0 (black) .. 1 (white). */
export function luminance(hex: string) {
  const c = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
}
export const rgb = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)).join(', ');

/** Near-black used for text on light fills. */
export const INK = '#1c1c1e';
/** Contrast of '#rrggbb' against white (1..21). */
const onWhite = (hex: string) => 1.05 / (luminance(hex) + 0.05);
/** Text and icons on an accent fill: white while white reads on it (3:1, bold UI text and
 *  icons), otherwise near-black -- a white or yellow accent made white labels vanish. */
export const onAccent = (accent: string): string => (onWhite(accent) >= 3 ? '#fff' : INK);
/** The accent drawn on a white surface (Liquid's "on" tiles): itself if it shows there. */
export const accentInk = (accent: string): string => (onWhite(accent) >= 3 ? accent : INK);

// Readability floors. Without blur a see-through surface puts text straight on the
// wallpaper, so it must stay mostly solid; cards must stay visible as buttons.
export const FLOOR = { dock: 0.45, panel: 0.7, card: 0.3, high: 0.85 };

/** What is really drawn for a theme: scheme, whether Windows blurs, and the opacities after
 *  the readability floors (`raised` says which ones a floor lifted, for the settings hints). */
export function resolve(t: Theme, env: { transparency: boolean; prefersDark: boolean; behind?: Behind | null }) {
  const blurred = t.nativeBlur && env.transparency;
  // A custom background picks the text colour that reads on it, whatever the mode says.
  const dark = t.bg ? luminance(t.bg) < 0.4 : t.mode === 'dark' || (t.mode === 'auto' && env.prefersDark);
  const high = t.contrast === 'high';
  const tintL = t.bg ? luminance(t.bg) : dark ? luminance('#1c1c20') : luminance('#f6f6fa');
  // See-through surfaces over the wallpaper: tint at least enough for 4.5:1 text.
  const wall = env.transparency && env.behind ? tintFor(dark, tintL, env.behind) : 0;
  const floor = (v: number, min: number) => Math.max(v, high ? FLOOR.high : 0, blurred ? 0 : min, wall);
  const dock = floor(t.opacity, FLOOR.dock);
  const panel = floor(t.panelOpacity, FLOOR.panel);
  const card = Math.max(t.cardOpacity, FLOOR.card);
  return { dark, blurred, dock, panel, card, wall,
    raised: { dock: dock > t.opacity, panel: panel > t.panelOpacity, card: card > t.cardOpacity } };
}

/** Luminance of the darkest and brightest part of the wallpaper behind the dock (Rust: backdrop.rs). */
export interface Behind { min: number; max: number }

/** Smallest tint opacity that keeps text 4.5:1 over the worst part of the wallpaper.
 *  White text needs the mix at most 0.183 luminance; dark text (#1c1c1e) at least 0.229.
 *  ponytail: mixes luminance linearly and ignores the cards' own tint, so it errs solid. */
export function tintFor(darkScheme: boolean, tintL: number, b: Behind) {
  const [target, wallL] = darkScheme ? [0.183, b.max] : [0.229, b.min];
  const need = darkScheme ? wallL > target : wallL < target;
  if (!need || Math.abs(wallL - tintL) < 1e-6) return 0;
  return Math.min(1, Math.max(0, (wallL - target) / (wallL - tintL)));
}

/** How much of Windows' own milky Acrylic layer covers what's behind, measured on Windows 11
 *  (light / dark). Apps can't change it; the preview draws it so it matches the dock. */
export const ACRYLIC_BASE = { light: 0.85, dark: 0.75 };

/** Outer window corners Windows draws around a material (it can only do 0, 4 or 8 px);
 *  mirrors the mapping in platform/windows/appbar.rs. */
export const windowRadius = (r: number, material: boolean) => (!material ? r : r === 0 ? 0 : r <= 5 ? 4 : 8);
