// Holidays for one clock: loaded once per window (Rust caches the calendar for a week),
// filtered to the chosen state ('auto' = the state of your shared location).
import { ipc, type Holiday } from '../../lib/ipc';
import { useLocation } from '../../lib/location.svelte';
import type { ClockSettings } from './index';
import { stateOf, upcoming, ymd } from './holidays';

const all = $state<{ items: Holiday[] }>({ items: [] });
let started = false;

function start() {
  if (started) return;
  started = true;
  const load = () => ipc.holidaysGet().then((r) => (all.items = r.items)).catch(() => {});
  load();
  setInterval(load, 6 * 3600_000);
}

export function holidaysFor(settings: () => ClockSettings, now: () => Date) {
  start();
  const here = useLocation();
  const state = $derived(settings().holidayState === 'auto' ? stateOf(here.s?.place?.name) : settings().holidayState);
  const today = $derived(ymd(now()));
  const list = $derived(upcoming(all.items, state, today));
  return {
    get state() { return state; },
    get today() { return today; },
    get list() { return list; },
  };
}
