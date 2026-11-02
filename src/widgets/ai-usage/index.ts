import { Gauge } from '@lucide/svelte';
import { defineWidget } from '../types';
import Pill from './Pill.svelte';
import Popup from './Popup.svelte';
import SettingsForm from './SettingsForm.svelte';

export interface AiUsageSettings { days: number; show: 'left' | 'used'; /** Dock rows the user unticked (Session, Weekly, or a model name). */ hidden: string[] }

export default defineWidget<AiUsageSettings>({
  id: 'ai-usage',
  name: 'AI Usage',
  description: 'Claude plan limits left (5-hour and weekly) and daily Claude Code tokens',
  category: 'ai',
  icon: Gauge,
  sizes: ['small', 'medium'],
  defaultSize: 'medium',
  defaults: { days: 30, show: 'left', hidden: [] },
  Pill,
  Popup: { component: Popup, width: 420, height: 520 },
  SettingsForm,
});
