<script lang="ts">
  // Gmail address + App Password. The password is checked with Gmail, then kept in Windows
  // Credential Manager by Rust; it never goes into the widget's settings.
  import { ExternalLink } from '@lucide/svelte';
  import { untrack } from 'svelte';
  import { ipc } from '../../lib/ipc';

  let { email, onSaved }: { email: string; onSaved: (email: string) => void } = $props();

  // Starts from the saved address; typing here doesn't change the setting until sign-in works.
  let addr = $state(untrack(() => email));
  let pw = $state('');
  let busy = $state(false);
  let error = $state('');

  const submit = async (e: SubmitEvent) => {
    e.preventDefault();
    busy = true; error = '';
    try {
      const a = addr.trim().toLowerCase();
      await ipc.gmailSignIn(a, pw);
      pw = '';
      onSaved(a);
    } catch (err) { error = String(err); }
    finally { busy = false; }
  };
</script>

<form class="signin" onsubmit={submit}>
  <label>Gmail address
    <input type="email" autocomplete="username" placeholder="name@gmail.com" bind:value={addr} required />
  </label>
  <label>App Password
    <input type="password" autocomplete="off" placeholder="abcd efgh ijkl mnop" bind:value={pw} required />
  </label>
  {#if error}<p class="err">{error}</p>{/if}
  <button class="go" type="submit" disabled={busy || !addr.trim() || !pw.trim()}>{busy ? 'Checking…' : 'Sign in'}</button>

  <ol class="help">
    <li>Turn on 2-Step Verification in your Google Account.</li>
    <li>Make an App Password (any name, e.g. "GlassBar") and paste it here.</li>
  </ol>
  <button type="button" class="link" onclick={() => ipc.gmailAppPasswords()}><ExternalLink size={12} /> Open Google App Passwords</button>
  <p class="note">GlassBar only reads your inbox. It never sends, deletes or marks mail as read. The App Password stays in Windows Credential Manager.</p>
</form>

<style>
  .signin { display: flex; flex-direction: column; gap: var(--s-2); }
  label { display: flex; flex-direction: column; gap: var(--s-1); font-size: 12px; color: var(--fg-dim); }
  input { height: 36px; padding: 0 var(--s-3); border-radius: var(--r-inner); background: var(--bg-2); border: 1px solid var(--line-soft); color: var(--fg); font: inherit; font-size: 13px; }
  input:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
  .go { height: 36px; border-radius: var(--r-inner); background: var(--accent); color: var(--on-accent); font-weight: 600; font-size: 13px; }
  .go:disabled { opacity: 0.5; }
  .err { margin: 0; font-size: 12px; color: #ff453a; }
  .help { margin: var(--s-1) 0 0; padding-left: 18px; font-size: 12px; color: var(--fg-dim); line-height: 1.5; }
  .link { display: inline-flex; align-items: center; gap: 6px; align-self: flex-start; font-size: 12px; color: var(--accent); }
  .link:hover { text-decoration: underline; }
  .note { margin: 0; font-size: 11px; color: var(--fg-3); line-height: 1.45; }
</style>
