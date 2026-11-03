// App Folders: the pure part (which folder, which apps, in what order). Unit tested.
import type { Pin } from '../apps/index';
import { keyFor, nameOf } from '../apps/groups';

/** One folder on the dock. An empty name shows no label. */
export interface Folder { id: string; name: string; apps: Pin[] }

/** Popup keys: a folder opened by a click (focused) or by hovering (closes on mouse-out). */
export const FOLDER = '__folder:';
export const FOLDER_HOVER = '__folderh:';
export const folderKey = (hover: boolean, instance: string, id: string) => `${hover ? FOLDER_HOVER : FOLDER}${instance}|${id}`;
/** The folder id of the "All folders" panel (opened from +n or the widget around the folders). */
export const ALL = '*';
/** The folders shown on the dock: the first `max` (2-4), and how many more there are. */
export const shownCount = (max: number | undefined) => Math.min(4, Math.max(2, Math.round(max ?? 3)));
export function visibleFolders(list: Folder[], max: number | undefined): { shown: Folder[]; more: number } {
  const n = shownCount(max);
  return list.length <= n ? { shown: list, more: 0 } : { shown: list.slice(0, n), more: list.length - n };
}
export function parseFolderKey(key: string | null | undefined): { instance: string; id: string; hover: boolean } | null {
  if (!key) return null;
  const hover = key.startsWith(FOLDER_HOVER);
  if (!hover && !key.startsWith(FOLDER)) return null;
  const [instance, id] = key.slice(hover ? FOLDER_HOVER.length : FOLDER.length).split('|');
  return instance && id ? { instance, id, hover } : null;
}

/** Which program a pin is: matches its running windows (and shares their icon). */
export const pinKey = (p: Pin) => keyFor(p.path, p.aumid);
/** Which app a pin is: the program *and* its arguments. "Developer Command Prompt" and
 *  "Command Prompt" are both cmd.exe, but two different apps in a folder. */
export const pinId = (p: Pin) => `${pinKey(p)}|${(p.args ?? '').trim().toLowerCase()}`;
export const pinName = (p: Pin) => p.name ?? nameOf(p.path);

/** Only programs and shortcuts can go in a folder. */
export const isApp = (path: string) => /\.(exe|lnk|url|appref-ms)$/i.test(path);

/** Add apps to the end, skipping ones already in the folder. */
export const addApps = (f: Folder, pins: Pin[]): Folder => {
  const have = new Set(f.apps.map(pinId));
  const fresh = pins.filter((p) => { const k = pinId(p); if (have.has(k)) return false; have.add(k); return true; });
  return { ...f, apps: [...f.apps, ...fresh] };
};
export const removeApp = (f: Folder, key: string): Folder => ({ ...f, apps: f.apps.filter((p) => pinId(p) !== key) });
/** Move the app `key` to position `to` (clamped). */
export function moveApp(f: Folder, key: string, to: number): Folder {
  const from = f.apps.findIndex((p) => pinId(p) === key);
  if (from < 0) return f;
  const apps = [...f.apps];
  const [p] = apps.splice(from, 1);
  apps.splice(Math.max(0, Math.min(apps.length, to)), 0, p);
  return { ...f, apps };
}
/** Replace folder `id` in the list. */
export const replace = (list: Folder[], f: Folder) => list.map((x) => (x.id === f.id ? f : x));
export const newFolder = (): Folder => ({ id: `f${Date.now().toString(36)}${Math.random().toString(36).slice(2, 5)}`, name: '', apps: [] });

/** An app dragged from the App Launcher and released on a folder tile: the launcher sends
 *  this DOM event to the tile (`data-folder-drop`); the folder adds the app. Both widgets live
 *  in the dock's own page, so a plain event is all it takes. */
export const ADD_APP = 'glassbar:add-app';
/** Set on <html> while an App Launcher icon is being dragged, so folders can light up. */
export const APP_DRAG = 'appDrag';
