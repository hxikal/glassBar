import { events, ipc, type WaterToday } from '../../lib/ipc';

/** Today's water, kept current: Rust sends `water:changed` after every add or remove (and each
 *  reminder); it is asked again at every midnight, and whenever `settings()` (the goal) changes.
 *  `now` ticks every 20 s for the reminder countdown. */
export function liveWater(settings: () => unknown) {
  let v = $state<WaterToday | null>(null);
  let now = $state(Date.now());
  $effect(() => { const t = setInterval(() => (now = Date.now()), 20_000); return () => clearInterval(t); });
  const load = () => ipc.waterToday().then((t) => (v = t)).catch(() => {});
  $effect(() => {
    JSON.stringify(settings()); // the goal, weight or mode changed: the goal is Rust's to work out
    load();
  });
  $effect(() => {
    const off = events.waterChanged((t) => (v = t));
    // A new day starts at midnight: ask again then, and arm for the next one.
    let t: ReturnType<typeof setTimeout>;
    const arm = () => {
      const now = new Date();
      const ms = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1).getTime() - now.getTime();
      t = setTimeout(() => { load(); arm(); }, ms + 1000);
    };
    arm();
    // After sleep the timer may be late: a page coming back into view asks again.
    const vis = () => { if (document.visibilityState === 'visible') load(); };
    document.addEventListener('visibilitychange', vis);
    return () => { off.then((f) => f()); clearTimeout(t); document.removeEventListener('visibilitychange', vis); };
  });
  return { get v() { return v; }, get now() { return now; } };
}
