<script lang="ts">
  // Settings › About › Updates: check, download and install a new version.
  import { relaunch } from '@tauri-apps/plugin-process';
  import { check, type Update } from '@tauri-apps/plugin-updater';

  let update = $state<Update | null>(null);
  let status = $state('');
  let busy = $state(false);
  let progress = $state(0);

  async function checkNow() {
    busy = true;
    status = 'Checking…';
    try {
      update = await check();
      status = update ? `Version ${update.version} is available.` : 'GlassBar is up to date.';
    } catch (e) {
      status = `Couldn't check for updates: ${e}`;
    }
    busy = false;
  }

  async function install() {
    if (!update) return;
    busy = true;
    let total = 0, got = 0;
    try {
      await update.downloadAndInstall((e) => {
        if (e.event === 'Started') total = e.data.contentLength ?? 0;
        if (e.event === 'Progress') { got += e.data.chunkLength; progress = total ? got / total : 0; }
        if (e.event === 'Finished') status = 'Installing…';
      });
      await relaunch();
    } catch (e) {
      status = `Update failed: ${e}`;
      busy = false;
    }
  }
</script>

<div class="group" id="updates">
  <div class="field">
    <span>{status || 'GlassBar checks for updates when it starts and once a day.'}</span>
    <button class="btn" disabled={busy} onclick={checkNow}>Check now</button>
  </div>
  {#if update}
    {#if update.body}<p class="hint notes">{update.body}</p>{/if}
    <div class="field">
      <span>{busy && progress ? `Downloading… ${Math.round(progress * 100)}%` : 'GlassBar will restart after installing.'}</span>
      <button class="btn" disabled={busy} onclick={install}>Install & restart</button>
    </div>
  {/if}
</div>

<style>
  .notes { white-space: pre-wrap; }
</style>
