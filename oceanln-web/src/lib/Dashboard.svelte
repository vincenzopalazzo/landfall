<script lang="ts">
  import Icon from "./ui/Icon.svelte";
  import StatsGrid from "./StatsGrid.svelte";
  import { app } from "./store.svelte";
  import { MCP } from "./data";
  import { isTauri } from "./tauri";

  // ── MCP (separate process: `oceanln-mcp` proxies the REST API) ──
  // Per PR E, MCP no longer lives in this process. The user starts the
  // `oceanln-mcp` binary alongside `oceanln-httpd`; this panel tells them
  // (a) the shell command to start it, and (b) the URL to paste into
  // their MCP client.
  //
  // Hidden in the Tauri desktop build: desktop talks to Rust over native
  // IPC and never runs httpd, so MCP isn't reachable from a desktop-only
  // install. Run `oceanln-httpd` + `oceanln-mcp` separately if you want
  // AI access. (PR B Dockerfile bundles both for hosted deploys.)
  const showMcp = !isTauri();
  const mcpClientUrl = MCP.clientUrl;
  const mcpRunCmd = $derived(
    MCP.runCmd
      .replace("{httpdBase}", app.base.replace(/\/$/, ""))
      // Include `--httpd-token` only when httpd is in authed mode (we
      // see a bearer in `app.token`). Without the flag, `oceanln-mcp`
      // proxies every tool unauthed, so anything except get_health
      // would 401 against an authed httpd. The Codex review on PR E
      // caught this: copy-paste must produce a working setup.
      .replace("{authFlag}", app.token ? ` --httpd-token ${app.token}` : ""),
  );
  let copiedRun = $state(false);
  let copiedUrl = $state(false);
  function copyRun() {
    navigator.clipboard?.writeText(mcpRunCmd).catch(() => {});
    copiedRun = true;
    setTimeout(() => (copiedRun = false), 1600);
  }
  function copyUrl() {
    navigator.clipboard?.writeText(mcpClientUrl).catch(() => {});
    copiedUrl = true;
    setTimeout(() => (copiedUrl = false), 1600);
  }
  const offerValue = $derived(app.offer || "lno1…(create a wallet first)");
</script>

<div class="db-wrap">
  <StatsGrid title="Payout dashboard" sub="OCEAN mining payouts for your address(es)" />

  <div class="db-grid2">
    <div class="db-panel db-offercard">
      <div class="db-panel-h"><h3><Icon name="wallet" size={15} /> Your offer</h3></div>
      <div class="db-panel-b">
        <p class="od">{app.offerDescription || "OCEAN mining payouts"}</p>
        <p class="ov">{offerValue}</p>
        <span class="db-chip muted">BOLT12 · reusable</span>
      </div>
    </div>

    {#if showMcp}
      <div class="db-panel">
        <div class="db-panel-h">
          <h3><Icon name="spark" size={15} /> AI access (MCP)</h3>
          <span class="db-chip">Separate process</span>
        </div>
        <div class="db-panel-b">
          <p class="db-mcp-intro">
            Wire your wallet into Claude Code (or any MCP client) so the AI
            can read your OCEAN payout state. Step 1: start the
            <code>oceanln-mcp</code> proxy in another terminal:
          </p>
          <div class="db-code">
            <code><span class="pre">$ </span>{mcpRunCmd}</code>
            <button class="wz-copybtn {copiedRun ? 'copied' : ''}" onclick={copyRun}><Icon name={copiedRun ? "check" : "copy"} size={13} />{copiedRun ? "Copied" : "Copy"}</button>
          </div>
          <p class="db-mcp-intro" style="margin-top:14px">
            Step 2: add this URL to your MCP client (no header needed):
          </p>
          <div class="db-code">
            <code>{mcpClientUrl}</code>
            <button class="wz-copybtn {copiedUrl ? 'copied' : ''}" onclick={copyUrl}><Icon name={copiedUrl ? "check" : "copy"} size={13} />{copiedUrl ? "Copied" : "Copy"}</button>
          </div>
          <p class="db-cap-h">What it can do</p>
          <ul class="db-caps">
            {#each MCP.tools as t}
              <li class="db-cap"><Icon name="check" size={15} />{t}</li>
            {/each}
          </ul>
          <div class="db-clients"><Icon name="lock" size={13} /> {MCP.cannot}</div>
        </div>
      </div>
    {/if}
  </div>
</div>
