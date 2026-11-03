import { Moon } from '@lucide/svelte';
import { defineWidget } from '../types';
import Pill from './Pill.svelte';
import Popup from './Popup.svelte';
import SettingsForm from './SettingsForm.svelte';

/** azanSound: what plays at prayer time: the opening of the adzan, all of it, or the normal chime.
 *  azanVoice: which bundled recording (lib/sound ADHANS); the user's own azanFile replaces it.
 *  pauseMusic: pause other playing media during the adzan and resume it afterwards. */
/** showHijri / hijriOnDock: the Hijri date in the panel / as the large pill's second line.
 *  showTasbih / tasbihOnDock: the Tasbih counter in the panel / joined to the prayer pill.
 *  tasbihTarget and dhikr: what is counted (Tasbih Fatimah has its own 33 · 33 · 34). */
export interface SolatSettings { showHijri: boolean; hijriOnDock: boolean; showTasbih: boolean; tasbihOnDock: boolean; tasbihTarget: number; dhikr: import('./tasbih').DhikrKey; zone: string; showImsak: boolean; alerts: boolean; azanSound: 'short' | 'full' | 'chime'; azanVoice: import('../../lib/sound').AdhanVoice; azanFile: string; pauseMusic: boolean; /** Adzan volume, 0-100 %. */ azanVolume: number; ackKey: string }

export default defineWidget<SolatSettings>({
  id: 'solat',
  name: 'Waktu Solat',
  description: 'Prayer times (JAKIM zones, worldwide fallback), countdown, alerts, Hijri date and a Tasbih counter',
  category: 'info',
  icon: Moon,
  sizes: ['small', 'medium', 'large'],
  defaultSize: 'medium',
  defaults: { showHijri: true, hijriOnDock: false, showTasbih: true, tasbihOnDock: true, tasbihTarget: 33, dhikr: 'subhanallah', zone: 'auto', showImsak: false, alerts: true, azanSound: 'short', azanVoice: 'madina', azanFile: '', pauseMusic: true, azanVolume: 100, ackKey: '' },
  Pill,
  Popup: { component: Popup, width: 320, height: 420 },
  SettingsForm,
  perf: { low: 'Prayer-time glow stays still' },
});
