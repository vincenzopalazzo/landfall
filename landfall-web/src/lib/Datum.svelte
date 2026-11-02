<script lang="ts">
  // Live view of the user's own DATUM Gateway (its public JSON API). The page
  // never contacts the gateway itself: it sends no CORS headers and the
  // desktop CSP allows no such host, so the snapshot comes from the backend
  // (`GET /datum/overview` over httpd, the `datum_overview` command on
  // desktop), which talks only to the URL saved in Settings.
  import Icon from "./ui/Icon.svelte";
  import Callout from "./ui/Callout.svelte";
  import { app, client } from "./store.svelte";
  import type { DatumOverview } from "./api";
  import {
    fmtBtc, fmtDifficulty, fmtInt, fmtThs, fmtUptime, isCleartextRemote, rejectPct, shortHex,
  } from "./datumFormat";

  const REFRESH_MS = 15_000;

  let ov = $state<DatumOverview | null>(null);
  let error = $state("");
  let loading = $state(true);
  let updatedAt = $state(0);
  // Drops a response that lands after a newer request, or after the gateway
  // changed (same pattern as StatsGrid.load).
  let reqId = 0;

  async function load() {
    const myId = ++reqId;
    try {
      const r = await client().datumOverview();
      if (myId !== reqId) return;
      ov = r;
      error = "";
      updatedAt = Date.now();
    } catch (e) {
      if (myId !== reqId) return;
      // Keep the last good snapshot on screen (marked stale) rather than
      // blanking the tab on one dropped poll.
      error = e instanceof Error ? e.message : String(e);
    } finally {
      if (myId === reqId) loading = false;
    }
  }

  // (Re)load on mount and whenever the gateway changes. The previous
  // gateway's numbers are dropped so they never show under the new URL.
  $effect(() => {
    void app.datumUrl;
    ov = null;
    error = "";
    loading = true;
    void load();
    const id = setInterval(() => void load(), REFRESH_MS);
    return () => {
      clearInterval(id);
      reqId++;
    };
  });

  const chip = $derived.by(() => {
    if (error) return { cls: "warn", label: "Unreachable" };
    if (!ov) return { cls: "muted", label: "Connecting" };
    if (ov.stats.ready === "Ready") return { cls: "ok", label: "Ready" };
    if (ov.stats.ready === "Non-Pooled Mode and Ready") return { cls: "muted", label: "Solo mode" };
    return { cls: "warn", label: ov.stats.ready || "Not ready" };
  });
  const pooled = $derived(!!ov && ov.stats.pool_host !== "" && ov.stats.pool_host !== "N/A");
  const hashrate = $derived(fmtThs(ov?.server.estimated_hashrate ?? 0));
  const rejected = $derived(
    ov ? rejectPct(ov.stats.accepted_shares_diff, ov.stats.rejected_shares_diff) : 0,
  );
  const weightPct = $derived(
    ov?.job && ov.job.limits.weight > 0 ? (ov.job.weight / ov.job.limits.weight) * 100 : 0,
  );
  const coinbaseTotal = $derived((ov?.coinbase ?? []).reduce((sum, o) => sum + o.sats, 0));
  const cleartext = $derived(isCleartextRemote(app.datumUrl));
  const updated = $derived(updatedAt ? new Date(updatedAt).toLocaleTimeString() : "");
</script>

<div class="db-wrap">
  <div class="db-top">
    <div>
      <h1>DATUM Gateway</h1>
      <p class="sub">Block templates from your own node, mined through your gateway at <span class="dt-url">{app.datumUrl}</span></p>
    </div>
    <span class="db-chip {chip.cls}"><span class="db-dot {chip.cls === 'ok' ? 'pulse' : ''}"></span>{chip.label}</span>
  </div>

  {#if cleartext}
    <Callout kind="warn" icon="warn">
      This gateway is reached over plain <b>http://</b> outside your local network, so its
      stats travel unencrypted. Put it behind a VPN or a TLS reverse proxy.
    </Callout>
  {/if}

  {#if error}
    <Callout kind="danger" icon="warn">
      {error}{#if ov}<br />Showing the last answer, from {updated}.{/if}
    </Callout>
  {/if}

  {#if loading && !ov}
    <p class="dt-empty">Contacting the gateway…</p>
  {:else if ov}
    <div class="db-stats">
      <div class="db-stat">
        <div class="l">Hashrate</div>
        <div class="v accent">{hashrate.v}<span class="u">{hashrate.u}</span></div>
        <div class="f">estimated from accepted shares</div>
      </div>
      <div class="db-stat">
        <div class="l">Workers</div>
        <div class="v">{fmtInt(ov.server.total_work_subscriptions)}<span class="u">subscribed</span></div>
        <div class="f">{fmtInt(ov.server.total_connections)} connections · {fmtInt(ov.server.active_thread)} threads</div>
      </div>
      <div class="db-stat">
        <div class="l">Shares accepted</div>
        <div class="v">{fmtInt(ov.stats.accepted_shares)}</div>
        <div class="f">{fmtInt(ov.stats.rejected_shares)} rejected · {rejected.toFixed(2)}% by difficulty</div>
      </div>
      <div class="db-stat">
        <div class="l">Uptime</div>
        <div class="v">{fmtUptime(ov.stats.uptime)}</div>
        <div class="f">updated {updated}</div>
      </div>
    </div>

    <div class="db-grid2">
      <div class="db-panel">
        <div class="db-panel-h">
          <h3><Icon name="link" size={16} /> Pool</h3>
          <span class="db-chip muted">{pooled ? "Pooled" : "Not pooled"}</span>
        </div>
        <div class="db-panel-b">
          <dl class="dt-kv">
            <dt>Status</dt><dd>{ov.stats.ready || "—"}</dd>
            <dt>Pool</dt><dd>{pooled ? ov.stats.pool_host : "None (solo)"}</dd>
            <dt>Pool tag</dt><dd>{ov.stats.pool_tag || "—"}</dd>
            <dt>Miner tag</dt><dd>{ov.stats.miner_tag || "—"}</dd>
            <dt>Min difficulty</dt><dd>{fmtInt(ov.stats.pool_min_diff)}</dd>
            <dt>Pool key</dt><dd title={ov.stats.pool_pub_key}>{shortHex(ov.stats.pool_pub_key) || "—"}</dd>
          </dl>
        </div>
      </div>

      <div class="db-panel">
        <div class="db-panel-h">
          <h3><Icon name="btc" size={16} /> Current job</h3>
          {#if ov.job}<span class="meta">height {fmtInt(ov.job.block_height)}</span>{/if}
        </div>
        <div class="db-panel-b">
          {#if ov.job}
            <dl class="dt-kv">
              <dt>Block reward</dt><dd>{fmtBtc(ov.job.block_value)} BTC</dd>
              <dt>Transactions</dt><dd>{fmtInt(ov.job.tx_count)}</dd>
              <dt>Weight</dt><dd>{weightPct.toFixed(1)}% of limit</dd>
              <dt>Difficulty</dt><dd>{fmtDifficulty(ov.job.block_difficulty)}</dd>
              <dt>Previous block</dt>
              <dd>
                <a href="https://mempool.space/block/{encodeURIComponent(ov.job.previous_block)}" target="_blank" rel="noreferrer" style="color:var(--wiz-accent)">{shortHex(ov.job.previous_block)}</a>
              </dd>
            </dl>
          {:else}
            <p class="dt-empty">Waiting for the first block template from your node.</p>
          {/if}
        </div>
      </div>
    </div>

    <div class="db-panel">
      <div class="db-panel-h">
        <h3><Icon name="wallet" size={16} /> Coinbase outputs</h3>
        <span class="meta">{ov.coinbase.length} {ov.coinbase.length === 1 ? "output" : "outputs"}</span>
      </div>
      {#if ov.coinbase.length}
        <table class="db-table">
          <thead><tr><th>Address</th><th class="r">Share</th><th class="r">Amount</th></tr></thead>
          <tbody>
            {#each ov.coinbase as o (o.address)}
              <tr>
                <td class="dt-addr">
                  {o.address}
                  {#if app.miningAddress && o.address === app.miningAddress}<span class="db-status settled" style="margin-left:8px">your payout address</span>{/if}
                </td>
                <td class="r">{coinbaseTotal ? ((o.sats / coinbaseTotal) * 100).toFixed(2) : "0.00"}%</td>
                <td class="r amt">{fmtBtc(o.sats)} BTC</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <div class="db-panel-b"><p class="dt-empty">No coinbase outputs yet.</p></div>
      {/if}
    </div>

    {#if ov.threads.length}
      <div class="db-panel">
        <div class="db-panel-h"><h3><Icon name="enclave" size={16} /> Stratum threads</h3></div>
        <table class="db-table">
          <thead><tr><th>Thread</th><th class="r">Connections</th><th class="r">Subscribed</th><th class="r">Hashrate</th></tr></thead>
          <tbody>
            {#each ov.threads as t (t.id)}
              {@const hr = fmtThs(t.approx_hashrate)}
              <tr>
                <td>{t.id}</td>
                <td class="r">{fmtInt(t.connection_count)}</td>
                <td class="r">{fmtInt(t.subscription_count)}</td>
                <td class="r">{hr.v} {hr.u}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</div>
