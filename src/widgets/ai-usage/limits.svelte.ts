import { ipc, type AiLimits } from '../../lib/ipc';

/** Claude plan limits, re-asked every 5 min (Rust caches for 5 min too). */
export function liveLimits() {
  const s = $state<{ l: AiLimits | null; error: string; loading: boolean }>({ l: null, error: '', loading: true });
  const load = (force = false) => {
    s.loading = true;
    return ipc.aiLimits(force)
      .then((l) => { s.l = l; s.error = ''; })
      .catch((e) => { s.error = String(e); s.l = null; })
      .finally(() => (s.loading = false));
  };
  $effect(() => {
    load();
    const t = setInterval(load, 5 * 60_000);
    return () => clearInterval(t);
  });
  return Object.assign(s, { refresh: () => load(true) });
}
