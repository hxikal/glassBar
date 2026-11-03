<script lang="ts">
  // Laragon's services (start/stop, Start all / Stop all), then what runs outside Laragon:
  // database Windows services (admin prompt), Docker Desktop and its containers.
  import { Play, RefreshCw, Square } from '@lucide/svelte';
  import { ipc, type DevService } from '../../lib/ipc';
  import type { DevStackSettings } from './index';
  import { liveStack, poll } from './live.svelte';

  let { settings, report }: { settings: DevStackSettings; report: (e: unknown) => void } = $props();
  const stack = liveStack(() => settings.root, 3_000);
  const other = poll(() => ipc.devOther(), 5_000);
  let busy = $state<string[]>([]);
  const main = $derived((stack.st?.services ?? []).filter((s) => s.enabled));

  /** Run `job`, show a spinner on `key` for `ms`, then re-read. */
  async function act(key: string, job: () => Promise<unknown>, ms = 1500) {
    busy = [...busy, key]; report('');
    try { await job(); } catch (e) { report(e); }
    setTimeout(async () => { await Promise.all([stack.load(), other.load()]); busy = busy.filter((b) => b !== key); }, ms);
  }
  const toggle = (s: DevService, start = !s.running) => s.running !== start && act(s.id, () => ipc.devToggle(settings.root, s.id, start));
  const all = (start: boolean) => main.forEach((s) => toggle(s, start));
  const docker = $derived(other.v?.docker ?? null);
</script>

{#snippet button(key: string, running: boolean, label: string, onclick: () => void, disabled = false)}
  {@const b = busy.includes(key)}
  <button class="act" class:stop={running} disabled={b || disabled} {onclick} aria-label="{running ? 'Stop' : 'Start'} {label}" title="{running ? 'Stop' : 'Start'} {label}">
    {#if b}<RefreshCw size={13} class="spin" />{:else if running}<Square size={13} />{:else}<Play size={13} />{/if}
  </button>
{/snippet}

<div class="gtitle"><span class="dim">Laragon</span></div>
{#if !stack.st}
  <p class="empty">Reading Laragon…</p>
{:else if !stack.st.found}
  <p class="empty">Laragon isn't in <b>{settings.root}</b>. Set its folder in Settings › Widgets › Dev Stack.</p>
{:else}
  <div class="group">
    {#each stack.st.services as s (s.id)}
      <div class="svc" class:on={s.running}>
        <i class="dot"></i>
        <div class="col">
          <b>{s.name}</b>
          <span class="dim small ellipsis">{s.version} · port {s.port}{s.enabled ? '' : ' · not used by Laragon'}</span>
        </div>
        {@render button(s.id, s.running, s.name, () => toggle(s))}
      </div>
    {/each}
  </div>
  {#if main.length}
    <div class="row2">
      <button class="tile" onclick={() => all(true)} disabled={main.every((s) => s.running)}>Start all</button>
      <button class="tile" onclick={() => all(false)} disabled={main.every((s) => !s.running)}>Stop all</button>
    </div>
  {/if}
{/if}

{#if other.v}
  {#if other.v.services.length}
    <div class="gtitle"><span class="dim">Windows services</span><span class="dim small">asks for admin</span></div>
    <div class="group">
      {#each other.v.services as s (s.name)}
        <div class="svc" class:on={s.running} class:pending={s.pending}>
          <i class="dot"></i>
          <div class="col">
            <b class="ellipsis" title={s.display}>{s.display}</b>
            <span class="dim small ellipsis">{s.pending ? 'Changing…' : s.running ? 'Running' : 'Stopped'} · {s.name}</span>
          </div>
          {@render button(`svc:${s.name}`, s.running, s.display, () => act(`svc:${s.name}`, () => ipc.devService(s.name, !s.running), 4000), s.pending)}
        </div>
      {/each}
    </div>
  {/if}

  {#if docker}
    <div class="gtitle"><span class="dim">Docker</span></div>
    <div class="group">
      <div class="svc" class:on={docker.running}>
        <i class="dot"></i>
        <div class="col"><b>Docker Desktop</b><span class="dim small">{docker.running ? `${docker.containers.filter((c) => c.running).length}/${docker.containers.length} containers running` : 'Not running'}</span></div>
        {#if !docker.running}{@render button('docker', false, 'Docker Desktop', () => act('docker', () => ipc.devDocker('desktop'), 5000))}{/if}
      </div>
      {#each docker.containers as c (c.id)}
        <div class="svc" class:on={c.running}>
          <i class="dot"></i>
          <div class="col">
            <b class="ellipsis">{c.name}</b>
            <span class="dim small ellipsis" title="{c.image} · {c.status} {c.ports}">{c.image} · {c.status}</span>
          </div>
          {@render button(`ct:${c.id}`, c.running, c.name, () => act(`ct:${c.id}`, () => ipc.devDocker(c.running ? 'stop' : 'start', c.id), 2000))}
        </div>
      {/each}
    </div>
  {/if}
{/if}

<style>
  .row2 { display: grid; grid-template-columns: 1fr 1fr; gap: var(--s-2); margin-top: var(--s-2); }
</style>
