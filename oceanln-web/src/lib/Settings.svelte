<script lang="ts">
  import Icon from "./ui/Icon.svelte";
  import { app, refreshHealth } from "./store.svelte";
  import { isTauri } from "./tauri";
  let open = $state(false);

  // Local drafts, committed to the store on change (blur / Enter) rather than
  // per keystroke. `App.svelte` re-bootstraps whenever `app.base` / `app.token`
  // change, and a re-bootstrap against another server resets the wizard — so a
  // live binding would wipe an un-backed-up phrase while the user is still
  // typing the token (QA-212).
  let base = $state(app.base);
  let token = $state(app.token);
  function commit() {
    if (base.trim() !== app.base) app.base = base.trim();
    if (token !== app.token) app.token = token;
    void refreshHealth();
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === "Enter") (e.target as HTMLInputElement).blur();
  }
</script>

<button class="tw-btn" onclick={() => (open = !open)} aria-label="settings"><Icon name="cog" size={18} /></button>

{#if open}
  <div class="tw-panel">
    {#if !isTauri()}
      <!-- Desktop (Tauri) talks to the backend in-process over IPC — there's no
           base URL or bearer token to configure, so this block is browser-only. -->
      <h4>Connection</h4>
      <input class="wz-input" style="font-size:12px;padding:8px 10px;margin-bottom:6px" bind:value={base} placeholder="http://127.0.0.1:7762" onchange={commit} onkeydown={onKey} />
      <input class="wz-input" style="font-size:12px;padding:8px 10px" bind:value={token} placeholder="bearer token" type="password" onchange={commit} onkeydown={onKey} />
      <p style="font-size:11px;margin:6px 0 0;color:{app.serverUp ? '#22c55e' : '#71717a'}">
        {app.serverUp ? "server reachable" : "server not reached"} · applied when you leave the field
      </p>
    {/if}

    <h4>Explainers</h4>
    <div class="tw-row">
      <button class={app.density === "guided" ? "on" : ""} onclick={() => (app.density = "guided")}>Guided</button>
      <button class={app.density === "concise" ? "on" : ""} onclick={() => (app.density = "concise")}>Concise</button>
    </div>
  </div>
{/if}
