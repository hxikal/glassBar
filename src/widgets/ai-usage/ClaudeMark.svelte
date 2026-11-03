<script lang="ts">
  // Claude-style starburst in Claude's orange, drawn inline (no logo asset in the app).
  let { size = 16 }: { size?: number } = $props();
  // 11 rays with slightly uneven lengths and angles, like the hand-drawn mark.
  const rays = [
    [0, 10], [31, 8.4], [66, 9.6], [97, 8], [131, 9.8], [163, 8.6],
    [196, 9.4], [228, 8.2], [261, 10], [294, 8.4], [327, 9.2],
  ];
</script>

<svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true">
  {#each rays as [a, len] (a)}
    <line x1="12" y1="12" x2="12" y2={12 - len} transform="rotate({a} 12 12)"
      stroke="#d97757" stroke-width="2.4" stroke-linecap="round" />
  {/each}
</svg>
