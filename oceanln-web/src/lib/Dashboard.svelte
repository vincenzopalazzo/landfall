<script lang="ts">
  import Icon from "./ui/Icon.svelte";
  import { app } from "./store.svelte";
  import { DASH, MCP } from "./data";

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
  <div class="db-top">
    <div>
      <h1>Payout dashboard</h1>
      <p class="sub">Monitoring your OCEAN Lightning payouts · <span style="color:#52525b">illustrative data</span></p>
    </div>
    <span class="db-chip ok"><span class="db-dot pulse"></span>{DASH.node.status}</span>
  </div>

  <div class="db-stats">
    <div class="db-stat"><div class="l">Pending</div><div class="v accent">{DASH.pendingSats}<span class="u">sats</span></div></div>
    <div class="db-stat"><div class="l">Total paid</div><div class="v">{DASH.totalBtc}<span class="u">BTC</span></div></div>
    <div class="db-stat"><div class="l">Payouts</div><div class="v">{DASH.payoutsCount}</div></div>
    <div class="db-stat"><div class="l">Next ETA</div><div class="v">{DASH.nextEta}</div></div>
  </div>

  <div class="db-panel">
    <div class="db-panel-h">
      <h3><Icon name="bolt" size={15} /> Recent payouts</h3>
      <span class="meta">{DASH.node.enclave} · up {DASH.node.uptime}</span>
    </div>
    <table class="db-table">
      <thead><tr><th>Time</th><th class="r">Amount</th><th class="r">Status</th></tr></thead>
      <tbody>
        {#each DASH.payouts as p}
          <tr>
            <td>{p.time}</td>
            <td class="r amt">{p.sats}{p.sats !== "—" ? " sats" : ""}</td>
            <td class="r"><span class="db-status {p.status === 'settled' ? 'settled' : 'inflight'}">{p.status}</span></td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

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
        <h3><Icon name="spark" size={15} /> AI access (MCP)</h3>
        <button class="db-toggle {mcpOn ? 'on' : ''}" onclick={() => (mcpOn = !mcpOn)} aria-label="toggle MCP"><span class="knob"></span></button>
      </div>
      <div class="db-panel-b">
        <p class="db-mcp-intro">
          Connect your payout node to Claude Code or any MCP client. <span style="color:#52525b">(Illustrative — `oceanln mcp serve` is not implemented yet.)</span>
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
