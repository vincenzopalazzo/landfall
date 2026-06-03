<script lang="ts">
  import Icon from "./ui/Icon.svelte";
  import { app, refreshHealth } from "./store.svelte";
  let open = $state(false);
</script>

<button class="tw-btn" onclick={() => (open = !open)} aria-label="settings"><Icon name="cog" size={18} /></button>

{#if open}
  <div class="tw-panel">
    <h4>Connection</h4>
    <input class="wz-input" style="font-size:12px;padding:8px 10px;margin-bottom:6px" bind:value={app.base} placeholder="http://127.0.0.1:7762" onchange={refreshHealth} />
    <input class="wz-input" style="font-size:12px;padding:8px 10px" bind:value={app.token} placeholder="bearer token" type="password" />
    <p style="font-size:11px;margin:6px 0 0;color:{app.serverUp ? '#22c55e' : '#71717a'}">
      {app.serverUp ? "server reachable" : "server not reached"}
    </p>

    <h4>Explainers</h4>
    <div class="tw-row">
      <button class={app.density === "guided" ? "on" : ""} onclick={() => (app.density = "guided")}>Guided</button>
      <button class={app.density === "concise" ? "on" : ""} onclick={() => (app.density = "concise")}>Concise</button>
    </div>

    <h4>Accent</h4>
    <div class="tw-row">
      <button class={app.accent === "orange" ? "on" : ""} onclick={() => (app.accent = "orange")}>Orange</button>
      <button class={app.accent === "blue" ? "on" : ""} onclick={() => (app.accent = "blue")}>Blue</button>
    </div>
  </div>
{/if}
