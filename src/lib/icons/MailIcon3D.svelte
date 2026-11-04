<script lang="ts">
  // Envelope with depth, a sibling of the Notes / To-do icons (same light, shadow and gloss)
  // in sky blue, so the red unread badge stands out on it.
  let { size = 20 }: { size?: number } = $props();
  const id = $props.id();
</script>

<svg class="i3d" width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
  <defs>
    <linearGradient id="{id}-body" x1="0" y1="0" x2="0.35" y2="1">
      <stop offset="0" stop-color="#7CD3FF" />
      <stop offset="0.5" stop-color="#2E9BFF" />
      <stop offset="1" stop-color="#1764E8" />
    </linearGradient>
    <linearGradient id="{id}-gloss" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#fff" stop-opacity="0.6" />
      <stop offset="1" stop-color="#fff" stop-opacity="0" />
    </linearGradient>
    <linearGradient id="{id}-paper" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff" />
      <stop offset="1" stop-color="#DCEBFF" />
    </linearGradient>
    <linearGradient id="{id}-flap" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#F4F9FF" />
      <stop offset="1" stop-color="#C9DEFB" />
    </linearGradient>
    <filter id="{id}-sh" x="-30%" y="-30%" width="160%" height="170%">
      <feDropShadow dx="0" dy="1.6" stdDeviation="1.4" flood-color="#0a3a8c" flood-opacity="0.4" />
    </filter>
    <filter id="{id}-in" x="-20%" y="-20%" width="140%" height="160%">
      <feDropShadow dx="0" dy="0.8" stdDeviation="0.6" flood-color="#0a3a8c" flood-opacity="0.35" />
    </filter>
  </defs>
  <g filter="url(#{id}-sh)">
    <rect x="4.5" y="4.5" width="23" height="23" rx="6" fill="url(#{id}-body)" />
    <path d="M10.5 4.5h11a6 6 0 0 1 6 6v1H4.5v-1a6 6 0 0 1 6-6z" fill="url(#{id}-gloss)" />
    <rect x="4.5" y="4.5" width="23" height="23" rx="6" fill="none" stroke="#0B3F9E" stroke-opacity="0.35" stroke-width="0.8" />
    <!-- envelope: paper, side folds, then the closed flap on top -->
    <g filter="url(#{id}-in)">
      <rect x="8.2" y="10.6" width="15.6" height="11.2" rx="1.8" fill="url(#{id}-paper)" />
      <path d="M8.6 21.2l5.6-4.6M23.4 21.2l-5.6-4.6" stroke="#9DBCE8" stroke-width="0.9" stroke-linecap="round" />
      <path d="M8.4 11.4a1.6 1.6 0 0 1 1.5-0.8h12.2a1.6 1.6 0 0 1 1.5 0.8L17.3 16.7a2 2 0 0 1-2.6 0z" fill="url(#{id}-flap)" stroke="#9DBCE8" stroke-width="0.6" stroke-linejoin="round" />
    </g>
  </g>
</svg>

<style>
  .i3d { display: block; flex: none; overflow: visible; transition: transform 0.3s var(--spring); }
  :global(.pill:hover) .i3d, :global(.side:hover) .i3d { transform: translateY(-1px) rotate(-3deg) scale(1.08); }
  :global(.pill:active) .i3d { transform: scale(0.9); }
</style>
