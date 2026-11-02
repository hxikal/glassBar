<script lang="ts" module>
  // The real Windows icon for a file (cached per type in Rust; per path here).
  import { events, ipc } from '../../lib/ipc';
  const cache = new Map<string, Promise<string | null>>();
  events.boostFree(() => cache.clear());
  const iconOf = (path: string) => { if (!cache.has(path)) cache.set(path, ipc.fileIcon(path).catch(() => null)); return cache.get(path)!; };
</script>

<script lang="ts">
  import { File } from '@lucide/svelte';
  let { path, size = 20 }: { path: string; size?: number } = $props();
  let src = $state<string | null>(null);
  $effect(() => { const p = path; src = null; iconOf(p).then((s) => { if (p === path) src = s; }); });
</script>

{#if src}<img {src} alt="" width={size} height={size} draggable="false" />{:else}<File {size} class="dim" />{/if}

<style>
  img { display: block; flex: none; object-fit: contain; }
</style>
