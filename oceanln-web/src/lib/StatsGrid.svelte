<script lang="ts">
  // Live OCEAN payout stats + recent payouts, keyed by the user's payout
  // address(es). Self-loading and reusable: the full Lightning dashboard renders
  // it expanded; the Profile renders it `compact` so stats sit next to the
  // profile ("home / control center" — see the Console design). Data is the real
  // public OCEAN API (see ocean.ts), not mock values.
  import Icon from "./ui/Icon.svelte";
  import { app } from "./store.svelte";
  import { ocean, btcToSats, hashesToThs, num, type Payout, type PoolStat } from "./ocean";

  let {
    compact = false,
    title = "Payout dashboard",
    sub = "OCEAN mining payouts",
  }: { compact?: boolean; title?: string; sub?: string } = $props();

  let loading = $state(true);
  let netError = $state("");
  let payoutsError = $state(false); // earnpay failed (independently of statsnap)
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
      const [snaps, eps] = await Promise.all([
        Promise.allSettled(addrs.map((a) => ocean.statsnap(a))),
        Promise.allSettled(addrs.map((a) => ocean.earnpay(a))),
      ]);
      if (myId !== reqId) return; // superseded

      let unpaid = 0,
        hr = 0,
        est = 0;
      // Track stats and payout failures separately: earnpay can fail while
      // statsnap succeeds (and vice versa), and each drives a different state.
      let statFail = false,
        payoutFail = false;
      for (const s of snaps) {
        if (s.status === "fulfilled") {
          unpaid += btcToSats(s.value.unpaid);
          hr += hashesToThs(s.value.hashrate_300s);
          est += btcToSats(s.value.estimated_payout_next_block);
        } else if (!isNoSuchUser(s.reason)) {
          statFail = true;
        }
      }

      const all: Payout[] = [];
      for (const e of eps) {
        if (e.status === "fulfilled") all.push(...(e.value.payouts ?? []));
        else if (!isNoSuchUser(e.reason)) payoutFail = true;
      }
      all.sort((a, b) => num(b.ts) - num(a.ts));

      unpaidSats = unpaid;
      hashrateThs = hr;
      estNextSats = est;
      payouts = all;
      totalPaidSats = all.reduce((sum, p) => sum + num(p.total_satoshis_net_paid), 0);
      active = hr > 0;

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
  function fmtTs(ts: string | number): string {
    const ms = typeof ts === "number" || /^\d+$/.test(String(ts)) ? Number(ts) * 1000 : Date.parse(String(ts));
    const d = new Date(ms);
    return Number.isNaN(d.getTime()) ? String(ts) : d.toISOString().slice(0, 16).replace("T", " ");
  }
  const txidShort = (t: string) => (t ? `${t.slice(0, 10)}…${t.slice(-6)}` : "—");
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
      {#if pool}· <span style="color:#52525b">{Number(pool.active_users).toLocaleString()} miners on the pool</span>{/if}
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
      No payouts yet. Once OCEAN pays your address, transactions appear here.
    </div>
  {/if}
</div>
