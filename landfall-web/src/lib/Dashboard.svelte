<script lang="ts">
  import Icon from "./ui/Icon.svelte";
  import NodeWallet from "./NodeWallet.svelte";
  import { app, refreshHealth } from "./store.svelte";
  import { MCP } from "./data";
  import { isTauri } from "./tauri";

  // ── MCP (separate process: `landfall-mcp` proxies the REST API) ──
  // Per PR E, MCP no longer lives in this process. The user starts the
  // `landfall-mcp` binary alongside `landfall-httpd`; this panel tells them
  // (a) the shell command to start it, and (b) the URL to paste into
  // their MCP client.
  //
  // Hidden in the Tauri desktop build: desktop talks to Rust over native
  // IPC and never runs httpd, so MCP isn't reachable from a desktop-only
  // install. Run `landfall-httpd` + `landfall-mcp` separately if you want
  // AI access. (PR B Dockerfile bundles both for hosted deploys.)
  const showMcp = !isTauri();
  const mcpClientUrl = MCP.clientUrl;
  const mcpRunCmd = $derived(
    MCP.runCmd
      .replace("{httpdBase}", app.base.replace(/\/$/, ""))
      // Include `--httpd-token` only when httpd is in authed mode (we
      // see a bearer in `app.token`). Without the flag, `landfall-mcp`
      // proxies every tool unauthed, so anything except get_health
      // would 401 against an authed httpd. The Codex review on PR E
      // caught this: copy-paste must produce a working setup.
      .replace("{authFlag}", app.token ? ` --httpd-token ${app.token}` : ""),
  );

  // Tauri build: health is in-process (always reachable). Browser build:
  // `app.serverUp` reflects the last `landfall-httpd` /health probe. Keep it
  // fresh while the dashboard is open so the "Node online" chip is live, not
  // a one-shot read from page load. `refreshHealth()` is cheap (a single
  // unauthed GET) and the interval is torn down on unmount.
  const nodeOnline = $derived(isTauri() ? true : app.serverUp);
  $effect(() => {
    if (isTauri()) return; // in-process — nothing to poll
    void refreshHealth();
    const id = setInterval(() => void refreshHealth(), 30_000);
    return () => clearInterval(id);
  });

  let copiedRun = $state(false);
  let copiedUrl = $state(false);
  let copiedOffer = $state(false);
  let copiedAddr = $state(false);
  function copy(value: string, set: (v: boolean) => void) {
    navigator.clipboard?.writeText(value).catch(() => {});
    set(true);
    setTimeout(() => set(false), 1600);
  }

  const offerValue = $derived(app.offer || "lno1…(create a wallet first)");
  const hasOffer = $derived(!!app.offer);
</script>

<div class="db-wrap">
  <div class="db-top">
    <div>
      <h1>Lightning payouts</h1>
      <p class="sub">Your OCEAN mining rewards, paid over Lightning — receive, track, and withdraw.</p>
    </div>
    {#if nodeOnline}
      <span class="db-chip ok"><span class="db-dot pulse"></span>Node online</span>
    {:else}
      <span class="db-chip muted"><span class="db-dot"></span>Node unreachable</span>
    {/if}
  </div>

  <NodeWallet />

  <div style="height:22px"></div>

  <div class="db-grid2">
    <!-- Your Lightning offer — the BOLT12 destination OCEAN pays into, plus
         the payout address it's bound to. Both copyable. -->
    <div class="db-panel db-offercard">
      <div class="db-panel-h">
        <h3><Icon name="bolt" size={16} /> Your Lightning offer</h3>
        <span class="db-chip muted">BOLT12</span>
      </div>
      <div class="db-panel-b">
        <p class="od">Description</p>
        <p class="ov">{app.offerDescription || "OCEAN mining payouts"}</p>

        <p class="od">Offer (lno1…)</p>
        <div class="db-code">
          <code style="max-height:70px;overflow:hidden">{offerValue}</code>
          {#if hasOffer}
            <button class="wz-copybtn {copiedOffer ? 'copied' : ''}" onclick={() => copy(app.offer, (v) => (copiedOffer = v))}>
              <Icon name={copiedOffer ? "check" : "copy"} size={13} />{copiedOffer ? "Copied" : "Copy"}
            </button>
          {/if}
        </div>

        <div style="height:12px"></div>

        <p class="od">Payout address</p>
        <div class="db-code">
          <code>{app.miningAddress || "— create a wallet first —"}</code>
          {#if app.miningAddress}
            <button class="wz-copybtn {copiedAddr ? 'copied' : ''}" onclick={() => copy(app.miningAddress, (v) => (copiedAddr = v))}>
              <Icon name={copiedAddr ? "check" : "copy"} size={13} />{copiedAddr ? "Copied" : "Copy"}
            </button>
          {/if}
        </div>
      </div>
    </div>

    {#if showMcp}
      <div class="db-panel db-offercard">
        <div class="db-panel-h">
          <h3><Icon name="link" size={16} /> AI access · MCP</h3>
          <span class="db-chip muted">Separate process</span>
        </div>
        <div class="db-panel-b">
          <p class="db-mcp-intro">
            Connect your payout node to <b style="color:#fafafa">Claude Code</b> or any
            MCP-compatible assistant. The AI can read your OCEAN payout state over a
            local, self-hosted server.
          </p>

          <p class="db-cap-h">Step 1 — start the proxy</p>
          <div class="db-code">
            <code><span class="pre">$ </span>{mcpRunCmd}</code>
            <button class="wz-copybtn {copiedRun ? 'copied' : ''}" onclick={() => copy(mcpRunCmd, (v) => (copiedRun = v))}>
              <Icon name={copiedRun ? "check" : "copy"} size={13} />{copiedRun ? "Copied" : "Copy"}
            </button>
          </div>

          <p class="db-cap-h">Step 2 — add this URL to your MCP client</p>
          <div class="db-code">
            <code>{mcpClientUrl}</code>
            <button class="wz-copybtn {copiedUrl ? 'copied' : ''}" onclick={() => copy(mcpClientUrl, (v) => (copiedUrl = v))}>
              <Icon name={copiedUrl ? "check" : "copy"} size={13} />{copiedUrl ? "Copied" : "Copy"}
            </button>
          </div>

          <p class="db-cap-h">What the assistant can do</p>
          <ul class="db-caps">
            {#each MCP.tools as t}
              <li class="db-cap"><Icon name="check" size={15} stroke={2} />{t}</li>
            {/each}
          </ul>

          <div class="db-clients"><Icon name="lock" size={13} /> {MCP.cannot}</div>
        </div>
      </div>
    {/if}
  </div>
</div>
