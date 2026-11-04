<script lang="ts">
  import { Trash2 } from '@lucide/svelte';
  import type { SettingsProps } from '../types';
  import type { GmailSettings } from './index';
  import { ipc } from '../../lib/ipc';
  import { accountsOf, colorOf, nameOf, type Account } from './accounts';
  import SignIn from './SignIn.svelte';

  let { settings, save }: SettingsProps<GmailSettings> = $props();
  const accounts = $derived(accountsOf(settings));
  let adding = $state(false);

  const write = (list: Account[]) => save({ accounts: list, email: '' });
  const rename = (i: number, name: string) => write(accounts.map((a, j) => (j === i ? { ...a, name: name.trim() } : a)));
  const remove = async (a: Account) => {
    await ipc.gmailSignOut(a.email).catch(() => {});
    write(accounts.filter((x) => x.email !== a.email));
  };
  const added = (email: string) => {
    if (!accounts.some((a) => a.email === email)) write([...accounts, { email, name: '' }]);
    adding = false;
  };
</script>

<div class="accts">
  {#each accounts as a, i (a.email)}
    <div class="acct">
      <span class="sw" style:background={colorOf(i)}></span>
      <span class="who"><b class="ellipsis">{a.email}</b>
        <input placeholder={nameOf({ ...a, name: '' })} value={a.name} aria-label="Tab name for {a.email}"
          onchange={(e) => rename(i, e.currentTarget.value)} />
      </span>
      <button class="btn ghost" onclick={() => remove(a)} title="Sign out and remove"><Trash2 size={14} /> Remove</button>
    </div>
  {:else}
    <p class="dim">No Gmail account yet.</p>
  {/each}
</div>

{#if adding || !accounts.length}
  <SignIn email="" onSaved={added} />
  {#if accounts.length}<button class="btn ghost" onclick={() => (adding = false)}>Cancel</button>{/if}
{:else}
  <button class="btn" onclick={() => (adding = true)}>Add account</button>
{/if}

<label>Notify me about new email
  <select value={settings.notify ?? 'smart'} onchange={(e) => save({ notify: e.currentTarget.value as GmailSettings['notify'] })}>
    <option value="smart">Primary and important only</option>
    <option value="all">All new email</option>
    <option value="off">Off (badge only)</option>
  </select>
</label>

<label>Full check every
  <select value={settings.every} onchange={(e) => save({ every: Number(e.currentTarget.value) })}>
    {#each [5, 10, 15, 30, 60] as m (m)}<option value={m}>{m} min</option>{/each}
  </select>
</label>
<p class="dim" style="font-size:12px">New email shows up within seconds (GlassBar keeps a connection open to Gmail); the full check is a backup. The tab name is shown in the panel (leave empty for the part before @). Opening an email in GlassBar only hides it from the list; it stays unread in Gmail.</p>

<style>
  .accts { display: flex; flex-direction: column; gap: 8px; margin-bottom: 8px; }
  .acct { display: flex; align-items: center; gap: 10px; padding: 8px 10px; border-radius: var(--r-inner); background: var(--bg-2); }
  .sw { flex: none; width: 10px; height: 10px; border-radius: 50%; }
  .who { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; font-size: 13px; }
  .who input { width: 160px; font-size: 12px; }
</style>
