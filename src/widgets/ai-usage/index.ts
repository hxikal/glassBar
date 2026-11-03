import { Gauge } from '@lucide/svelte';
import { defineWidget } from '../types';
import Pill from './Pill.svelte';
import Popup from './Popup.svelte';
import SettingsForm from './SettingsForm.svelte';

export interface AiUsageSettings {
  days: number;
  show: 'left' | 'used';
  /** Dock rows the user unticked (Session, Weekly, or a model name). */
  hidden: string[];
  /** Dock rows one per line, or side by side. */
  layout: 'stacked' | 'row';
  /** Faint empty bar behind each fill. */
  track: boolean;
}

export default defineWidget<AiUsageSettings>({
  id: 'ai-usage',
  name: 'AI Usage',
  description: 'Claude plan limits left (5-hour and weekly) and daily Claude Code tokens',
  category: 'ai',
  icon: Gauge,
  sizes: ['small', 'medium'],
  defaultSize: 'medium',
  defaults: { days: 30, show: 'used', hidden: ['Session', 'Fable', 'Opus', 'Sonnet'], layout: 'stacked', track: true },
  Pill,
  Popup: { component: Popup, width: 420, height: 520 },
  SettingsForm,
});
