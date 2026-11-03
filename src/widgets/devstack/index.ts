import { Server } from '@lucide/svelte';
import { defineWidget } from '../types';
import Pill from './Pill.svelte';
import Popup from './Popup.svelte';
import SettingsForm from './SettingsForm.svelte';

export interface DevStackSettings {
  /** Laragon's folder (holds laragon.exe). */
  root: string;
  /** More folders with JS projects (www is always searched). */
  projectDirs: string[];
}

export default defineWidget<DevStackSettings>({
  id: 'devstack',
  name: 'Dev Stack',
  description: 'Your local dev stack: Laragon services, localhost servers (Vite, Next…), npm dev servers, PHP/Node versions, Postgres and Docker',
  category: 'apps',
  icon: Server,
  sizes: ['small', 'medium'],
  defaultSize: 'medium',
  defaults: { root: 'C:\\laragon', projectDirs: [] },
  Pill,
  Popup: { component: Popup, width: 380, height: 600 },
  SettingsForm,
  perf: { normal: 'Checks Laragon every 10 s; the open panel checks its tab every 3–10 s' },
});
