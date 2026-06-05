<script lang="ts">
  import Icon from "./ui/Icon.svelte";
  import StatsGrid from "./StatsGrid.svelte";
  import { app } from "./store.svelte";
  import { MCP } from "./data";

  // ── MCP (local stdio server) ──
  let mcpOn = $state(true);
  let copied = $state(false);
  function copyCmd() {
    navigator.clipboard?.writeText(MCP.addCmd).catch(() => {});
    copied = true;
    setTimeout(() => (copied = false), 1600);
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

    <div class="db-panel">
      <div class="db-panel-h">
        <h3><Icon name="spark" size={15} /> AI access (local MCP)</h3>
        <button class="db-toggle {mcpOn ? 'on' : ''}" onclick={() => (mcpOn = !mcpOn)} aria-label="toggle MCP"><span class="knob"></span></button>
      </div>
      <div class="db-panel-b">
        <p class="db-mcp-intro">
          Run a <b>local</b> MCP server so Claude Code (or any MCP client) can read your payout
          status over stdio — nothing is hosted or exposed. <span style="color:#52525b">(`oceanln mcp serve` — coming soon.)</span>
        </p>
        <div class="db-code">
          <code><span class="pre">$ </span>{MCP.addCmd}</code>
          <button class="wz-copybtn {copied ? 'copied' : ''}" onclick={copyCmd}><Icon name={copied ? "check" : "copy"} size={13} />{copied ? "Copied" : "Copy"}</button>
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
  </div>
</div>
