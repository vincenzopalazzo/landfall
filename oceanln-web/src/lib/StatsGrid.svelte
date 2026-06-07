<script lang="ts">
  // Live OCEAN payout stats + recent payouts, keyed by the user's payout
  // address(es). Self-loading and reusable: the full Lightning dashboard renders
  // it expanded; the Profile renders it `compact` so stats sit next to the
  // profile ("home / control center" — see the Console design). Every value is
  // the real public OCEAN API (statsnap + earnpay + user_hashrate + pool_stat,
  // see ocean.ts) — no mock data.
  import Icon from "./ui/Icon.svelte";
  import { app } from "./store.svelte";
  import { ocean, btcToSats, num, type Earning, type Payout, type PoolStat } from "./ocean";

  let {
    compact = false,
    title = "Payout dashboard",
    sub = "OCEAN mining payouts",
  }: { compact?: boolean; title?: string; sub?: string } = $props();

  let loading = $state(true);
  let netError = $state("");
  let payoutsError = $state(false); // earnpay failed (independently of statsnap)
  // statsnap-derived
  let hr300 = $state(0); // hashes/sec, 5m window
  let unpaidSats = $state(0);
  let estPayoutSats = $state(0); // estimated_payout_next_block (what the user actually gets)
  let estEarnSats = $state(0); // estimated_earn_next_block (before fee/bonus)
  let tidesShares = $state(0);
  // user_hashrate-derived (richer: longer windows + live worker count)
  let workers = $state(0);
  let hr3600 = $state(0); // 1h
  let hr10800 = $state(0); // 3h — matches OCEAN's stats page "3hr average"
  let hr86400 = $state(0); // 24h
  let lastShareTs = $state(0);
  // earnpay-derived
  let totalPaidSats = $state(0);
  let blocksFound = $state(0); // count of payouts that are coinbase (generation) txns
  let payouts = $state<Payout[]>([]);
  // Per-block earnings credited to this address (separate from payouts:
  // earnings happen on every block where the user has shares in the TIDES
  // window; payouts only happen when accumulated unpaid >= the OCEAN
  // payout threshold). On a brand-new or low-hashrate address you'll see
  // earnings but no payouts.
  let earnings = $state<Earning[]>([]);
  // pool context
  let pool = $state<PoolStat | null>(null);
  let active = $state(false);

  // Derived totals (read-only views over the raw fields above).
  const lifetimeSats = $derived(unpaidSats + totalPaidSats);
  // Your TIDES window share as a percentage of the whole pool — matches the
  // "Share Log Percentage" column on ocean.xyz/stats.
  const sharePct = $derived.by(() => {
    if (!pool) return 0;
    const total = num(pool.current_tides_shares);
    if (total <= 0) return 0;
    return (tidesShares / total) * 100;
  });

  function addresses(): string[] {
    const fromProfile = (app.profile?.addresses ?? [])
      .map((a) => a.address)
      .filter((a) => /^(bc1|[13])/.test(a)); // drop placeholders like "— derived on next payout —"
    const list = fromProfile.length ? fromProfile : app.miningAddress ? [app.miningAddress] : [];
    return [...new Set(list)];
  }

  const isNoSuchUser = (e: unknown) => e instanceof Error && /no such user/i.test(e.message);

  // Monotonic request id: a newer load() supersedes an in-flight older one so a
  // slow response (or an address change mid-flight) can't clobber fresh state.
  let reqId = 0;

  async function load() {
    const addrs = addresses();
    if (!addrs.length) {
      loading = false;
      return;
    }
    const myId = ++reqId;
    loading = true;
    netError = "";
    payoutsError = false;
    try {
      const [snaps, eps, hrs] = await Promise.all([
        Promise.allSettled(addrs.map((a) => ocean.statsnap(a))),
        Promise.allSettled(addrs.map((a) => ocean.earnpay(a))),
        Promise.allSettled(addrs.map((a) => ocean.userHashrate(a))),
      ]);
      if (myId !== reqId) return; // superseded

      // statsnap: balances, 5m hashrate, TIDES shares, last share.
      let unpaid = 0,
        estPayout = 0,
        estEarn = 0,
        tides = 0,
        h5 = 0,
        lastShare = 0,
        statFail = false;
      for (const s of snaps) {
        if (s.status === "fulfilled") {
          unpaid += btcToSats(s.value.unpaid);
          estPayout += btcToSats(s.value.estimated_payout_next_block);
          estEarn += btcToSats(s.value.estimated_earn_next_block);
          tides += num(s.value.shares_in_tides);
          h5 += num(s.value.hashrate_300s);
          lastShare = Math.max(lastShare, num(s.value.lastest_share_ts));
        } else if (!isNoSuchUser(s.reason)) {
          statFail = true;
        }
      }

      // user_hashrate: live worker count + longer windows. Best-effort — a
      // brand-new address with no history can 404 here; that just yields zeros.
      let wk = 0,
        h1h = 0,
        h3h = 0,
        h24h = 0;
      for (const h of hrs) {
        if (h.status === "fulfilled") {
          wk += h.value.active_worker_count | 0;
          h1h += num(h.value.hashrate_3600s);
          h3h += num(h.value.hashrate_10800s);
          h24h += num(h.value.hashrate_86400s);
          lastShare = Math.max(lastShare, num(h.value.lastest_share_ts));
        }
      }

      // earnpay: payout history + per-block earnings (the OCEAN /stats page
      // surfaces both — we missed earnings before).
      const allPayouts: Payout[] = [];
      const allEarnings: Earning[] = [];
      let payoutFail = false;
      for (const e of eps) {
        if (e.status === "fulfilled") {
          allPayouts.push(...(e.value.payouts ?? []));
          allEarnings.push(...(e.value.earnings ?? []));
        } else if (!isNoSuchUser(e.reason)) {
          payoutFail = true;
        }
      }
      allPayouts.sort((a, b) => earnpayTs(b.ts) - earnpayTs(a.ts));
      allEarnings.sort((a, b) => earnpayTs(b.ts) - earnpayTs(a.ts));

      unpaidSats = unpaid;
      estPayoutSats = estPayout;
      estEarnSats = estEarn;
      tidesShares = tides;
      hr300 = h5;
      lastShareTs = lastShare;
      workers = wk;
      hr3600 = h1h;
      hr10800 = h3h;
      hr86400 = h24h;
      payouts = allPayouts;
      earnings = allEarnings;
      totalPaidSats = allPayouts.reduce((sum, p) => sum + num(p.total_satoshis_net_paid), 0);
      blocksFound = allPayouts.filter((p) => p.is_generation_txn).length;
      active = wk > 0 || h5 > 0;

      // Only a genuine network/server failure is an error; "no such user yet"
      // (a brand-new address with no OCEAN history) renders as an empty state.
      if (statFail && snaps.every((s) => s.status === "rejected")) {
        const r = snaps.find((s) => s.status === "rejected") as PromiseRejectedResult | undefined;
        netError = r?.reason instanceof Error ? r.reason.message : "couldn't reach OCEAN";
      }
      // earnpay can fail independently — surface it so an empty/partial payouts
      // table isn't silently mistaken for an accurate "no payouts yet".
      payoutsError = payoutFail;
      try {
        const ps = await ocean.poolStat();
        if (myId === reqId) pool = ps;
      } catch {
        /* pool context is best-effort */
      }
    } catch (e) {
      if (myId === reqId) netError = e instanceof Error ? e.message : String(e);
    } finally {
      if (myId === reqId) loading = false;
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

  const rowLimit = $derived(compact ? 5 : 12);
  const fmtSats = (n: number) => n.toLocaleString("en-US");
  const fmtInt = (n: number) => Math.round(n).toLocaleString("en-US");

  // Adaptive hashrate unit so small miners don't read "0.00 Th/s". Value and
  // unit are separate nodes so callers can style the unit.
  function fmtHr(hps: number): { v: string; u: string } {
    if (hps >= 1e12) return { v: (hps / 1e12).toFixed(2), u: "Th/s" };
    if (hps >= 1e9) return { v: (hps / 1e9).toFixed(2), u: "Gh/s" };
    if (hps >= 1e6) return { v: (hps / 1e6).toFixed(2), u: "Mh/s" };
    return { v: fmtInt(hps), u: "h/s" };
  }
  // Headline hashrate: show the shortest window that has data
  // (5m → 1h → 3h → 24h), labeled with that window. An intermittently-active
  // miner (e.g. shares this hour but none in the last 5m) otherwise reads a
  // misleading "0 h/s" here even though OCEAN's site shows a non-zero
  // longer-window average.
  const headlineHr = $derived(
    hr300 > 0
      ? { label: "Hashrate (5m)", ...fmtHr(hr300) }
      : hr3600 > 0
        ? { label: "Hashrate (1h)", ...fmtHr(hr3600) }
        : hr10800 > 0
          ? { label: "Hashrate (3h)", ...fmtHr(hr10800) }
          : hr86400 > 0
            ? { label: "Hashrate (24h)", ...fmtHr(hr86400) }
            : { label: "Hashrate (5m)", ...fmtHr(0) },
  );

  // Share percentage formatter: OCEAN shows up to ~7 decimal places for tiny
  // miners (e.g. "0.0000016%"). Use fixed-significant-digit precision so a
  // share of 0.0000016% reads as "0.0000016%" and a share of 12.34% reads
  // as "12.34%" — without an avalanche of insignificant zeros either way.
  function fmtPct(n: number): string {
    if (!Number.isFinite(n) || n <= 0) return "0";
    if (n >= 1) return n.toFixed(2);
    // Pick enough decimals to show 2 significant digits.
    const decimals = Math.min(8, Math.max(2, 2 - Math.floor(Math.log10(n))));
    return n.toFixed(decimals);
  }

  function relTime(sec: number): string {
    if (!sec) return "—";
    const d = Math.max(0, Math.floor(Date.now() / 1000 - sec));
    if (d < 90) return `${d}s ago`;
    if (d < 5400) return `${Math.round(d / 60)}m ago`;
    if (d < 129600) return `${Math.round(d / 3600)}h ago`;
    return `${Math.round(d / 86400)}d ago`;
  }
  // OCEAN serializes timestamps two different ways across endpoints:
  //   - payouts (numeric epoch seconds as string): "1700000000"
  //   - earnings (ISO-8601 datetime string)     : "2026-06-07T03:35:48"
  // A shared parser keeps sorting + formatting from silently breaking when
  // the shape differs from what each table expected.
  function earnpayTs(ts: string | number): number {
    if (typeof ts === "number") return ts * 1000;
    const s = String(ts);
    if (/^\d+$/.test(s)) return Number(s) * 1000;
    const parsed = Date.parse(s);
    return Number.isFinite(parsed) ? parsed : 0;
  }
  function fmtTs(ts: string | number): string {
    const ms = earnpayTs(ts);
    if (!ms) return String(ts);
    return new Date(ms).toISOString().slice(0, 16).replace("T", " ");
  }
  const txidShort = (t: string) => (t ? `${t.slice(0, 10)}…${t.slice(-6)}` : "—");
  const blockShort = (h: string) => (h ? `${h.slice(0, 10)}…${h.slice(-6)}` : "—");
</script>

<div class="db-top">
  <div>
    {#if compact}
      <h2 class="pf-stats-title">{title}</h2>
    {:else}
      <h1>{title}</h1>
    {/if}
    <p class="sub">
      {sub}
      {#if pool}
        · <span style="color:#52525b">{fmtInt(num(pool.active_users))} miners · diff {(num(pool.network_difficulty) / 1e12).toFixed(1)}T · block reward {num(pool.current_estimated_block_reward).toFixed(3)} BTC</span>
      {/if}
    </p>
  </div>
  <div style="display:flex;align-items:center;gap:10px">
    {#if loading}
      <span class="db-chip muted"><span class="wz-spinner"></span> Loading</span>
    {:else if active}
      <span class="db-chip live"><span class="db-dot pulse"></span>Mining</span>
    {:else}
      <span class="db-chip muted"><span class="db-dot"></span>Idle</span>
    {/if}
    <button class="wz-copybtn" onclick={() => load()} disabled={loading} title="Refresh">
      <Icon name="refresh" size={13} /> Refresh
    </button>
  </div>
</div>

{#if netError}
  <div class="wz-callout danger"><Icon name="warn" size={17} /><div class="ct">Couldn't load OCEAN data: {netError}</div></div>
{/if}

<!--
  Primary cards. Order mirrors ocean.xyz/stats so a user can scan the two
  side-by-side. "Lifetime" is the headline truth that the user is earning
  (unpaid + already paid); "Blocks found" is the rare-but-meaningful event.
  Earn-vs-payout next block: OCEAN exposes both because they differ — earn
  is what your shares would generate, payout is what gets sent after the
  TIDES window math.
-->
<div class="db-stats">
  <div class="db-stat"><div class="l">{headlineHr.label}</div><div class="v">{headlineHr.v}<span class="u">{headlineHr.u}</span></div></div>
  <div class="db-stat"><div class="l">Unpaid</div><div class="v accent">{fmtSats(unpaidSats)}<span class="u">sats</span></div></div>
  <div class="db-stat"><div class="l">Total paid</div><div class="v">{fmtSats(totalPaidSats)}<span class="u">sats</span></div></div>
  <div class="db-stat"><div class="l">Lifetime</div><div class="v">{fmtSats(lifetimeSats)}<span class="u">sats</span></div></div>
  <div class="db-stat"><div class="l">Est. payout next block</div><div class="v">{fmtSats(estPayoutSats)}<span class="u">sats</span></div></div>
  <div class="db-stat"><div class="l">Est. earn next block</div><div class="v">{fmtSats(estEarnSats)}<span class="u">sats</span></div></div>
  <div class="db-stat"><div class="l">Blocks found</div><div class="v">{fmtInt(blocksFound)}<span class="u">{blocksFound === 1 ? "block" : "blocks"}</span></div></div>
</div>

<!-- Secondary real metrics from user_hashrate + statsnap. -->
<div class="db-substats">
  <span class="ss"><span class="ss-l">Workers</span><span class="ss-v">{fmtInt(workers)}</span></span>
  <span class="ss"><span class="ss-l">1h avg</span><span class="ss-v">{fmtHr(hr3600).v} {fmtHr(hr3600).u}</span></span>
  <span class="ss"><span class="ss-l">3h avg</span><span class="ss-v">{fmtHr(hr10800).v} {fmtHr(hr10800).u}</span></span>
  <span class="ss"><span class="ss-l">24h avg</span><span class="ss-v">{fmtHr(hr86400).v} {fmtHr(hr86400).u}</span></span>
  <span class="ss"><span class="ss-l">TIDES shares</span><span class="ss-v">{fmtInt(tidesShares)}</span></span>
  <span class="ss"><span class="ss-l">Share %</span><span class="ss-v">{fmtPct(sharePct)}%</span></span>
  <span class="ss"><span class="ss-l">Last share</span><span class="ss-v">{relTime(lastShareTs)}</span></span>
</div>

<!--
  Recent earnings: per-block credits OCEAN attributes to this address.
  Distinct from payouts: an earning happens every block where the user's
  shares contributed (TIDES window); a payout only happens when accumulated
  unpaid clears OCEAN's minimum payout threshold. On a low-hashrate / new
  address you'll typically see earnings but no payouts for a while.
-->
<div class="db-panel">
  <div class="db-panel-h">
    <h3><Icon name="spark" size={15} /> Recent earnings</h3>
    <span class="meta">per-block credits · OCEAN TIDES</span>
  </div>
  {#if payoutsError}
    <div class="db-panel-b" style="color:#FFB300;font-size:12.5px;display:flex;gap:7px;align-items:center">
      <Icon name="warn" size={14} /> Couldn't load earnings history from OCEAN — this list may be incomplete. Try Refresh.
    </div>
  {/if}
  {#if earnings.length}
    <table class="db-table">
      <thead><tr><th>Time (UTC)</th><th>Block</th><th class="r">Earned</th></tr></thead>
      <tbody>
        {#each earnings.slice(0, rowLimit) as e}
          <tr>
            <td>{fmtTs(e.ts)}</td>
            <td>
              {#if e.block_hash}
                <a href={`https://mempool.space/block/${e.block_hash}`} target="_blank" rel="noreferrer" style="color:var(--wiz-accent)">{blockShort(e.block_hash)}</a>
              {:else}—{/if}
            </td>
            <td class="r amt">{fmtSats(num(e.satoshis_net_earned))} sats</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else if !loading && !payoutsError}
    <div class="db-panel-b" style="color:#71717a;font-size:13px">
      No earnings yet. Once your workers contribute shares in OCEAN's reward window, per-block credits will appear here.
    </div>
  {/if}
</div>

<div class="db-panel">
  <div class="db-panel-h">
    <h3><Icon name="bolt" size={15} /> Recent payouts</h3>
    <span class="meta">past 30 days · OCEAN TIDES</span>
  </div>
  {#if payoutsError}
    <div class="db-panel-b" style="color:#FFB300;font-size:12.5px;display:flex;gap:7px;align-items:center">
      <Icon name="warn" size={14} /> Couldn't load payout history from OCEAN — this list may be incomplete. Try Refresh.
    </div>
  {/if}
  {#if payouts.length}
    <table class="db-table">
      <thead><tr><th>Time (UTC)</th><th>Transaction</th><th class="r">Amount</th></tr></thead>
      <tbody>
        {#each payouts.slice(0, rowLimit) as p}
          <tr>
            <td>{fmtTs(p.ts)}</td>
            <td>
              {#if p.on_chain_txid}
                <a href={`https://mempool.space/tx/${p.on_chain_txid}`} target="_blank" rel="noreferrer" style="color:var(--wiz-accent)">{txidShort(p.on_chain_txid)}</a>
              {:else}—{/if}
              {#if p.is_generation_txn}<span class="db-status settled" style="margin-left:8px">coinbase</span>{/if}
            </td>
            <td class="r amt">{fmtSats(num(p.total_satoshis_net_paid))} sats</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else if !loading && !payoutsError}
    <div class="db-panel-b" style="color:#71717a;font-size:13px">
      {#if earnings.length}
        Earnings are accumulating (above), but no payouts yet. OCEAN sends a Lightning payout once your unpaid balance clears the minimum threshold.
      {:else}
        No payouts yet. Once OCEAN pays your address, transactions appear here.
      {/if}
    </div>
  {/if}
</div>
