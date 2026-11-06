// Native context menus.
// - A menu must stay alive until its click has been delivered, so the previous one is
//   freed only when the next one opens.
// - Every item gets a unique id (submenu items too). Tauri maps item id -> click handler;
//   when two menus shared an id ("bar:edit"), freeing the old menu deleted the new menu's
//   handler, so "Edit widgets" / "Settings…" worked once and then never again.
// - One menu at a time, and no menu resource is created or freed while one is open.
//   Tauri's popup() holds the webview's resource lock until the menu closes, while the
//   main thread runs the menu; a Menu.new/close arriving then waits for that lock on the
//   main thread, so the menu can never close and GlassBar hangs for good. This is why
//   separators are '-' here rather than PredefinedMenuItem objects made by the caller.
import { Menu, type IconMenuItemOptions, type MenuItemOptions } from '@tauri-apps/api/menu';

/** A plain item, a ticked item (`checked`), one with an icon (`icon`: PNG bytes), a submenu
 *  (`items`), or a separator ('-'). */
export type Item =
  | (Omit<MenuItemOptions, 'id'> & { checked?: boolean })
  | Omit<IconMenuItemOptions, 'id'>
  | { text: string; items: Item[]; enabled?: boolean }
  | '-';

let last: Menu | null = null;
let seq = 0;
let queue: Promise<unknown> = Promise.resolve();

function withIds(items: Item[], prefix: string): any[] {
  return items.map((it, i) => {
    if (it === '-') return { item: 'Separator' };
    const id = `${prefix}-${i}`;
    return 'items' in it ? { ...it, id, items: withIds(it.items, id) } : { ...it, id };
  });
}

async function open(items: Item[]) {
  const n = ++seq;
  const prev = last;
  last = await Menu.new({ items: withIds(items, `menu${n}`) });
  await prev?.close().catch(() => {});
  await last.popup();
}

/** Show a menu; resolves when it closes. Calls made while one is open wait their turn. */
export function showMenu(items: Item[]): Promise<void> {
  const run = queue.then(() => open(items));
  queue = run.catch(() => {});
  return run;
}
