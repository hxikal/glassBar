import { FolderOpen } from '@lucide/svelte';
import { defineWidget } from '../types';
import Pill from './Pill.svelte';
import type { Folder } from './folders';

export interface FoldersSettings {
  /** Several folders per widget, each its own tile on the dock. */
  folders: Folder[];
  /** Show each folder's name beside it on the dock (a folder with no name shows none). */
  showNames: boolean;
  /** Open a folder by hovering it, not only by clicking. */
  openOnHover: boolean;
  /** How many folders show on the dock (2-4); the rest are behind a "+n" chip. */
  maxShown: number;
}

export default defineWidget<FoldersSettings>({
  id: 'folders',
  name: 'App Folders',
  description: 'Group apps into folders, like on a phone',
  category: 'apps',
  icon: FolderOpen,
  sizes: ['small'],
  defaultSize: 'small',
  defaults: { folders: [{ id: 'f1', name: '', apps: [] }], showNames: true, openOnHover: false, maxShown: 3 },
  Pill,
});
