import { LayoutGrid } from '@lucide/svelte';
import { defineWidget } from '../types';
import Pill from './Pill.svelte';
import SettingsForm from './SettingsForm.svelte';

/** aumid: Store-app identity (Store apps can only be started through it). */
export interface Pin { path: string; args?: string; aumid?: string; /** shown name (web apps: the app, not the browser) */ name?: string }
export interface AppsSettings {
  pinned: Pin[];
  showTitles: boolean;
  /** Start & Search: 'inline' = left of the app icons, 'after' = right of them, 'separate' = their own widget. */
  startPlacement: 'inline' | 'after' | 'separate';
  /** Icons shown before a "+N" chip (0 = all); hovering the launcher shows them all. */
  maxIcons: number;
  searchStyle: 'icon' | 'bar';
  /** What the Search button opens: GlassBar's search (maths, units, currency, apps) or Windows Search. */
  searchOpens?: 'glassbar' | 'windows';
  /** The Windows logo on the Start button: blue (as Windows), the accent, the text colour,
   *  the classic four colours, or `startCustom`. */
  startColor?: 'blue' | 'accent' | 'text' | 'classic' | 'custom';
  startCustom?: string;
  /** Icon order chosen by dragging (program keys = lower-case paths). */
  order: string[];
  /** Mac-style: icons grow under the pointer, neighbours less (off by default). */
  magnify?: boolean;
  /** Largest size while magnified, 1.2-2.0. */
  magnifyScale?: number;
  /** Effect drawn when a window minimizes or is restored from the dock. */
  minimizeEffect?: 'genie' | 'scale' | 'suck' | 'none';
  /** The icon bounces while its app starts (off by default, like the other dock effects). */
  bounce?: boolean;
}

export default defineWidget<AppsSettings>({
  id: 'apps',
  name: 'App Launcher',
  description: 'Pinned and running apps, grouped by program',
  category: 'apps',
  icon: LayoutGrid,
  sizes: ['small', 'medium', 'large'],
  defaultSize: 'large',
  defaults: { pinned: [], showTitles: false, startPlacement: 'inline', searchStyle: 'icon', order: [], maxIcons: 0,
    magnify: false, magnifyScale: 1.6, minimizeEffect: 'none', bounce: false },
  Pill,
  SettingsForm,
});
