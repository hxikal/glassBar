// Laragon status, re-read every `ms` while the caller is mounted.
import { ipc, type DevStatus } from '../../lib/ipc';

export function liveStack(root: () => string, ms: number) {
  const s = $state<{ st: DevStatus | null }>({ st: null });
  const load = () => ipc.devStatus(root()).then((v) => (s.st = v)).catch(() => {});
  $effect(() => {
    root();
    load();
    const t = setInterval(load, ms);
    return () => clearInterval(t);
  });
  return { get st() { return s.st; }, load };
}
