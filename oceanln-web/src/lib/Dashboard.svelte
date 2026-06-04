<script lang="ts">
  import Icon from "./ui/Icon.svelte";
  import { app } from "./store.svelte";
  import { MCP } from "./data";
  import { ocean, btcToSats, hashesToThs, type Payout, type PoolStat } from "./ocean";

  // ── MCP (local stdio server) ──
  let mcpOn = $state(true);
  let copied = $state(false);
  function copyCmd() {
    navigator.clipboard?.writeText(MCP.addCmd).catch(() => {});
    copied = true;
    setTimeout(() => (copied = false), 1600);
  }
  const offerValue = $derived(app.offer || "lno1…(create a wallet first)");

  // ── OCEAN payout data, keyed by the user's payout address(es) ──
  let loading = $state(true);
  let netError = $state("");
  let unpaidSats = $state(0);
  let totalPaidSats = $state(0);
  let estNextSats = $state(0);
  let hashrateThs = $state(0);
  let active = $state(false);
  let payouts = $state<Payout[]>([]);
  let pool = $state<PoolStat | null>(null);

  function addresses(): string[] {
    const fromProfile = (app.profile?.addresses ?? [])
      .map((a) => a.address)
      .filter((a) => /^(bc1|[13])/.test(a)); // drop placeholders like "— derived on next payout —"
    const list = fromProfile.length ? fromProfile : app.miningAddress ? [app.miningAddress] : [];
    return [...new Set(list)];
  }

  const isNoSuchUser = (e: unknown) => e instanceof Error && /no such user/i.test(e.message);

  async function load() {
    const addrs = addresses();
    if (!addrs.length) {
      loading = false;
      return;
    }
    loading = true;
    netError = "";
    try {
      const [snaps, eps] = await Promise.all([
        Promise.allSettled(addrs.map((a) => ocean.statsnap(a))),
        Promise.allSettled(addrs.map((a) => ocean.earnpay(a))),
      ]);

      let unpaid = 0,
        hr = 0,
        est = 0,
        netFail = false;
      for (const s of snaps) {
        if (s.status === "fulfilled") {
          unpaid += btcToSats(s.value.unpaid);
          hr += hashesToThs(s.value.hashrate_300s);
          est += btcToSats(s.value.estimated_payout_next_block);
        } else if (!isNoSuchUser(s.reason)) {
          netFail = true;
        }
      }

      const all: Payout[] = [];
      for (const e of eps) {
        if (e.status === "fulfilled") all.push(...(e.value.payouts ?? []));
        else if (!isNoSuchUser(e.reason)) netFail = true;
      }
      all.sort((a, b) => Number(b.ts) - Number(a.ts));

      unpaidSats = unpaid;
      hashrateThs = hr;
      estNextSats = est;
      payouts = all;
      totalPaidSats = all.reduce((sum, p) => sum + Number(p.total_satoshis_net_paid), 0);
      active = hr > 0;

      // Only a genuine network/server failure is an error; "no such user yet"
      // (a brand-new address with no OCEAN history) renders as an empty state.
      if (netFail && snaps.every((s) => s.status === "rejected")) {
        const r = snaps.find((s) => s.status === "rejected") as PromiseRejectedResult | undefined;
        netError = r?.reason instanceof Error ? r.reason.message : "couldn't reach OCEAN";
      }
      try {
        pool = await ocean.poolStat();
      } catch {
        /* pool context is best-effort */
      }
    } catch (e) {
      netError = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  // Load on mount and whenever the address set changes.
  let lastKey = "";
  $effect(() => {
    const key = addresses().join(",");
    if (key !== lastKey) {
      lastKey = key;
      load();
    }
  });

  const fmtSats = (n: number) => n.toLocaleString("en-US");
  function fmtTs(ts: string | number): string {
    const ms = typeof ts === "number" || /^\d+$/.test(String(ts)) ? Number(ts) * 1000 : Date.parse(String(ts));
    const d = new Date(ms);
    return Number.isNaN(d.getTime()) ? String(ts) : d.toISOString().slice(0, 16).replace("T", " ");
  }
  const txidShort = (t: string) => (t ? `${t.slice(0, 10)}…${t.slice(-6)}` : "—");
</script>

<div class="db-wrap">
  <div class="db-top">
    <div>
      <h1>Payout dashboard</h1>
      <p class="sub">
        OCEAN mining payouts for your address{addresses().length > 1 ? "es" : ""}
        {#if pool}· <span style="color:#52525b">{Number(pool.active_users).toLocaleString()} miners on the pool</span>{/if}
      </p>
    </div>
    {#if loading}
      <span class="db-chip muted"><span class="wz-spinner"></span> Loading</span>
    {:else if active}
      <span class="db-chip live"><span class="db-dot pulse"></span>Mining</span>
    {:else}
      <span class="db-chip muted"><span class="db-dot"></span>Idle</span>
    {/if}
  </div>

  {#if netError}
    <div class="wz-callout danger"><Icon name="warn" size={17} /><div class="ct">Couldn't load OCEAN data: {netError}</div></div>
  {/if}

  <div class="db-stats">
    <div class="db-stat"><div class="l">Hashrate (5m)</div><div class="v">{hashrateThs.toFixed(2)}<span class="u">Th/s</span></div></div>
    <div class="db-stat"><div class="l">Unpaid</div><div class="v accent">{fmtSats(unpaidSats)}<span class="u">sats</span></div></div>
    <div class="db-stat"><div class="l">Total paid</div><div class="v">{fmtSats(totalPaidSats)}<span class="u">sats</span></div></div>
    <div class="db-stat"><div class="l">Est. next block</div><div class="v">{fmtSats(estNextSats)}<span class="u">sats</span></div></div>
  </div>

  <div class="db-panel">
    <div class="db-panel-h">
      <h3><Icon name="bolt" size={15} /> Recent payouts</h3>
      <span class="meta">past 30 days · OCEAN TIDES</span>
    </div>
    {#if payouts.length}
      <table class="db-table">
        <thead><tr><th>Time (UTC)</th><th>Transaction</th><th class="r">Amount</th></tr></thead>
        <tbody>
          {#each payouts.slice(0, 12) as p}
            <tr>
              <td>{fmtTs(p.ts)}</td>
              <td>
                {#if p.on_chain_txid}
                  <a href={`https://mempool.space/tx/${p.on_chain_txid}`} target="_blank" rel="noreferrer" style="color:var(--wiz-accent)">{txidShort(p.on_chain_txid)}</a>
                {:else}—{/if}
                {#if p.is_generation_txn}<span class="db-status settled" style="margin-left:8px">coinbase</span>{/if}
              </td>
              <td class="r amt">{fmtSats(Number(p.total_satoshis_net_paid))} sats</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else if !loading}
      <div class="db-panel-b" style="color:#71717a;font-size:13px">
        No payouts yet. Once OCEAN pays your address, transactions appear here.
      </div>
    {/if}
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
