import { Mail } from '@lucide/svelte';
import { defineWidget } from '../types';
import type { Account } from './accounts';
import Pill from './Pill.svelte';
import Popup from './Popup.svelte';
import SettingsForm from './SettingsForm.svelte';

export interface GmailSettings {
  /** Gmail accounts; each App Password is in Windows Credential Manager, not here. */
  accounts: Account[];
  /** Minutes between full checks (new mail itself arrives in realtime). */
  every: number;
  /** Which new mail raises a notification: Primary + important, all, or none. */
  notify: 'smart' | 'all' | 'off';
  /** Before multi-account: the one address (read through accountsOf()). */
  email?: string;
}

export default defineWidget<GmailSettings>({
  id: 'gmail',
  name: 'Gmail',
  description: 'Unread email from your Gmail accounts, sorted like Gmail, read right in the dock (never marked read in Gmail)',
  category: 'media',
  icon: Mail,
  // One size: the icon and its badge say it all.
  sizes: ['small'],
  defaultSize: 'small',
  defaults: { accounts: [], every: 15, notify: 'smart' },
  Pill,
  Popup: { component: Popup, width: 400, height: 560 },
  SettingsForm,
});
