import { Server } from '@lucide/svelte';
import { defineWidget } from '../types';
import Pill from './Pill.svelte';
import Popup from './Popup.svelte';
import SettingsForm from './SettingsForm.svelte';

export interface DevStackSettings {
  /** Laragon's folder (holds laragon.exe). */
  root: string;
}

export default defineWidget<DevStackSettings>({
  id: 'devstack',
  name: 'Dev Stack',
  description: 'Laragon at a glance: Apache, MySQL, Redis… running or not, start/stop, and your .test sites',
  category: 'apps',
  icon: Server,
  sizes: ['small', 'medium'],
  defaultSize: 'medium',
  defaults: { root: 'C:\\laragon' },
  Pill,
  Popup: { component: Popup, width: 340, height: 560 },
  SettingsForm,
  perf: { normal: 'Checks services every 10 s' },
});
