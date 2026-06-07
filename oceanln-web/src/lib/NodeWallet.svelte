<script lang="ts">
  // Node wallet — live balances (Lightning channel + on-chain), full node
  // activity (inbound + outbound, LN + on-chain) with OCEAN-payout
  // detection, and Send / Receive flows. Every value is real: balances and
  // activity come from the in-process Lexe node (via `client()`), the
  // BTC/USD rate from a live price feed. No mock data.
  import Icon from "./ui/Icon.svelte";
  import { app, client } from "./store.svelte";
  import type { Activity, NodeStatus } from "./api";
  import { btcUsd } from "./price";

  let status = $state<NodeStatus | null>(null);
  let acts = $state<Activity[]>([]);
  let loading = $state(true);
  let nodeErr = $state(""); // node_info failed (balances unavailable)
  let price = $state(0); // BTC/USD; 0 = unavailable → USD toggle hidden
  let unit = $state<"sats" | "usd">("sats");
  let filter = $state<"all" | "ocean" | "in" | "out">("all");
  let q = $state("");
  let modal = $state<null | "send" | "receive">(null);

  // Monotonic guard so a slow refresh can't clobber newer state.
  let reqId = 0;
  async function load() {
    const myId = ++reqId;
    loading = true;
    nodeErr = "";
    // Balances and activity load independently — a node that's still
    // provisioning may fail node_info but already have activity, or vice
    // versa. Neither rejection aborts the other.
    const [st, ac] = await Promise.all([
      client()
        .nodeStatus()
        .catch((e) => {
          if (myId === reqId) nodeErr = e instanceof Error ? e.message : String(e);
          return null;
        }),
      client()
        .activity(500)
        .catch(() => [] as Activity[]),
    ]);
    if (myId !== reqId) return;
    status = st;
    acts = ac;
    loading = false;
    btcUsd().then((p) => {
      if (myId === reqId) price = p;
    });
  }

  $effect(() => {
    load();
  });
  // Keep balances + activity live while the dashboard is open.
  $effect(() => {
    const id = setInterval(() => {
      if (!loading) load();
    }, 60_000);
    return () => clearInterval(id);
  });

  const totalReceived = $derived(
    acts.filter((a) => a.direction === "in" && a.status === "settled").reduce((s, a) => s + a.amount_sats, 0),
  );
  const oceanCount = $derived(acts.filter((a) => a.is_ocean).length);

  // ── formatting ──
  const commas = (n: number) => Math.round(n).toLocaleString("en-US");
  const usdOf = (sats: number) => (price > 0 ? (sats / 1e8) * price : 0);
  const fmtUsd = (n: number) =>
    "$" + n.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  const showUsd = $derived(unit === "usd" && price > 0);
  function big(sats: number): string {
    return showUsd ? fmtUsd(usdOf(sats)) : commas(sats);
  }
  function sub(sats: number): string {
    return unit === "usd" ? commas(sats) + " sats" : price > 0 ? fmtUsd(usdOf(sats)) : "";
  }

  // ── activity filtering + grouping ──
  const rows = $derived.by<Activity[]>(() =>
    acts.filter((a) => {
      if (filter === "ocean" && !a.is_ocean) return false;
      if (filter === "in" && a.direction !== "in") return false;
      if (filter === "out" && a.direction !== "out") return false;
      if (q.trim()) {
        const hay = ((a.counterparty || "") + " " + (a.note || "")).toLowerCase();
        if (!hay.includes(q.trim().toLowerCase())) return false;
      }
      return true;
    }),
  );
  const grouped = $derived(filter === "all" && !q.trim());
  const oceanRows = $derived(grouped ? rows.filter((a) => a.is_ocean) : []);
  const otherRows = $derived(grouped ? rows.filter((a) => !a.is_ocean) : rows);

  function partyName(a: Activity): string {
    if (a.counterparty) return a.counterparty;
    if (a.is_ocean) return "OCEAN";
    const verb = a.direction === "in" ? "received" : "sent";
    return (a.rail === "ln" ? "Lightning" : "On-chain") + " " + verb;
  }
  function relTime(ms: number): string {
    if (!ms) return "—";
    const d = Math.max(0, Math.floor((Date.now() - ms) / 1000));
    if (d < 90) return d + "s ago";
    if (d < 5400) return Math.round(d / 60) + "m ago";
    if (d < 129600) return Math.round(d / 3600) + "h ago";
    return Math.round(d / 86400) + "d ago";
  }
  function rowHref(a: Activity): string | null {
    if (a.payment_hash) return `https://ocean.xyz/info/tx/lightning/${a.payment_hash}`;
    if (a.txid) return `https://mempool.space/tx/${a.txid}`;
    return null;
  }

  // Capacity bar: spendable vs the rest of the channel balance.
  const capPct = $derived(
    status && status.lightning_total_sats > 0
      ? Math.max(2, Math.min(100, (status.lightning_sendable_sats / status.lightning_total_sats) * 100))
      : 0,
  );

  // ── Send flow ──
  let sendPayable = $state("");
  let sendAmount = $state(""); // sats, as typed
  let sendNote = $state("");
  let sendStep = $state<1 | 2 | "sending" | "done">(1);
  let sendErr = $state("");
  let sentSummary = $state<{ amount: number; id: string } | null>(null);
  const sendAmtSats = $derived(parseInt(sendAmount.replace(/[^0-9]/g, ""), 10) || 0);
  const sendReady = $derived(sendPayable.trim().length > 6);

  function openSend() {
    sendPayable = "";
    sendAmount = "";
    sendNote = "";
    sendStep = 1;
    sendErr = "";
    sentSummary = null;
    modal = "send";
  }
  async function confirmSend() {
    sendStep = "sending";
    sendErr = "";
    try {
      const r = await client().pay(
        sendPayable.trim(),
        sendAmtSats > 0 ? sendAmtSats : undefined,
        sendNote.trim() || undefined,
      );
      sentSummary = { amount: r.amount_sats || sendAmtSats, id: r.id };
      sendStep = "done";
      load(); // refresh balances + activity
    } catch (e) {
      sendErr = e instanceof Error ? e.message : String(e);
      sendStep = 2; // back to review so the user can retry/cancel
    }
  }

  // ── Receive flow ──
  // Only Lightning receive surfaces here: the reusable BOLT12 offer and a
  // freshly-minted BOLT11 invoice. There is deliberately NO on-chain tab —
  // `app.miningAddress` is the OCEAN signing/payout address, not the Lexe
  // node's on-chain deposit address, so showing it as a "deposit here"
  // address would be misleading (funds wouldn't credit the node wallet's
  // on-chain balance). Re-add an on-chain tab once the node exposes a real
  // on-chain receive address.
  let rcvTab = $state<"offer" | "invoice">("offer");
  let rcvAmount = $state("");
  let rcvInvoice = $state("");
  let rcvBusy = $state(false);
  let rcvErr = $state("");
  const rcvAmtSats = $derived(parseInt(rcvAmount.replace(/[^0-9]/g, ""), 10) || 0);
  function openReceive() {
    rcvTab = "offer";
    rcvAmount = "";
    rcvInvoice = "";
    rcvErr = "";
    modal = "receive";
  }
  async function genInvoice() {
    rcvBusy = true;
    rcvErr = "";
    try {
      rcvInvoice = await client().createInvoice(rcvAmtSats > 0 ? rcvAmtSats : undefined, "OCEAN node wallet");
    } catch (e) {
      rcvErr = e instanceof Error ? e.message : String(e);
    } finally {
      rcvBusy = false;
    }
  }
  const rcvValue = $derived(rcvTab === "offer" ? app.offer : rcvInvoice);

  let copied = $state("");
  function copy(value: string, tag: string) {
    if (!value) return;
    navigator.clipboard?.writeText(value).catch(() => {});
    copied = tag;
    setTimeout(() => (copied = ""), 1600);
  }
</script>

<!-- Action bar: unit toggle + Receive / Send -->
<div class="nw-bar">
  <span class="nw-bar-l">Node wallet</span>
  <span class="spacer"></span>
  {#if price > 0}
    <button class="nw-unit" onclick={() => (unit = unit === "sats" ? "usd" : "sats")}>
      <Icon name="refresh" size={14} /> {unit === "usd" ? "Show sats" : "Show USD"}
    </button>
  {/if}
  <button class="wz-btn subtle" onclick={openReceive}><Icon name="in" size={16} /> Receive</button>
  <button class="wz-btn" onclick={openSend}><Icon name="out" size={16} /> Send</button>
</div>

<!-- Balance cards -->
<div class="db-stats">
  <div class="db-stat">
    <div class="l"><Icon name="bolt" size={12} /> Lightning channel</div>
    <div class="v">{status ? big(status.lightning_total_sats) : "—"}{#if !showUsd && status}<span class="u"> sats</span>{/if}</div>
    {#if status}<div class="f">{sub(status.lightning_total_sats)}</div>{/if}
    {#if status && status.lightning_total_sats > 0}
      <div class="nw-cap">
        <div class="nw-cap-bar"><div class="nw-cap-out" style="width:{capPct}%"></div></div>
        <div class="nw-cap-leg">
          <span>{commas(status.lightning_sendable_sats)} spendable</span>
          <span>{commas(Math.max(0, status.lightning_total_sats - status.lightning_sendable_sats))} reserved</span>
        </div>
      </div>
    {/if}
  </div>
  <div class="db-stat">
    <div class="l"><Icon name="btc" size={12} /> On-chain</div>
    <div class="v">{status ? big(status.onchain_total_sats) : "—"}{#if !showUsd && status}<span class="u"> sats</span>{/if}</div>
    {#if status}<div class="f">{sub(status.onchain_total_sats)}</div>{/if}
  </div>
  <div class="db-stat">
    <div class="l"><Icon name="wallet" size={12} /> Total received</div>
    <div class="v">{big(totalReceived)}{#if !showUsd}<span class="u"> sats</span>{/if}</div>
    <div class="f">{sub(totalReceived)}</div>
  </div>
  <div class="db-stat">
    <div class="l"><Icon name="bolt" size={12} /> OCEAN payouts</div>
    <div class="v accent">{commas(oceanCount)}</div>
    <div class="f">received to your offer</div>
  </div>
</div>

{#if nodeErr}
  <div class="wz-callout warn" style="margin-bottom:18px"><Icon name="warn" size={17} /><div class="ct">Node balances unavailable: {nodeErr}. Activity still shown below.</div></div>
{/if}

<!-- Node activity -->
<div class="db-panel" style="margin-bottom:0">
  <div class="na-bar">
    <h3><Icon name="wallet" size={15} /> Node activity</h3>
    <div class="na-search">
      <Icon name="search" size={16} />
      <input placeholder="Search party, note…" bind:value={q} />
    </div>
  </div>
  <div class="na-filters">
    {#each [["all", "All", false], ["ocean", "OCEAN payouts", true], ["in", "Received", false], ["out", "Sent", false]] as [k, label, isOcean]}
      <button class="na-chip {isOcean ? 'ocean' : ''} {filter === k ? 'on' : ''}" onclick={() => (filter = k as typeof filter)}>
        {label}{#if k === "ocean"}<span class="ct">{oceanCount}</span>{/if}
      </button>
    {/each}
  </div>
  <div class="na-list">
    {#if loading && !acts.length}
      <div class="na-empty"><span class="wz-spinner"></span><div style="margin-top:10px">Loading activity…</div></div>
    {:else if !rows.length}
      <div class="na-empty"><Icon name="search" size={28} />No matching activity</div>
    {:else}
      {#if grouped && oceanRows.length}
        <div class="na-grp oc">OCEAN payouts<span class="gln"></span></div>
      {/if}
      {#each (grouped ? oceanRows : rows) as a (a.id)}
        {@render row(a)}
      {/each}
      {#if grouped && oceanRows.length && otherRows.length}
        <div class="na-grp">All other activity<span class="gln"></span></div>
      {/if}
      {#if grouped}
        {#each otherRows as a (a.id)}
          {@render row(a)}
        {/each}
      {/if}
    {/if}
  </div>
</div>

{#snippet row(a: Activity)}
  {@const inn = a.direction === "in"}
  {@const href = rowHref(a)}
  <a class="na-row {a.is_ocean ? 'ocean' : ''}" href={href ?? undefined} target={href ? "_blank" : undefined} rel="noreferrer">
    <span class="na-ic {a.status === 'failed' ? 'failed' : inn ? 'in' : 'out'}"><Icon name={inn ? "in" : "out"} size={18} /></span>
    <span class="na-meta">
      <span class="na-t">
        <span class="nm">{partyName(a)}</span>
        {#if a.is_ocean}<span class="oc-chip">OCEAN payout</span>{/if}
      </span>
      <span class="na-s">
        <Icon name={a.rail === "ln" ? "bolt" : "btc"} size={13} />{a.rail === "ln" ? "Lightning" : "On-chain"}
        <span class="sdot"></span>{relTime(a.finalized_at_ms)}
        {#if a.block_height}<span class="sdot"></span>block {a.block_height.toLocaleString("en-US")}{/if}
      </span>
    </span>
    <span class="na-amt">
      <span class="v {inn && a.amount_sats ? 'in' : ''}">
        {a.amount_sats ? (inn ? "+" : "−") + big(a.amount_sats) : "—"}{#if a.amount_sats && !showUsd}<span class="u"> sats</span>{/if}
      </span>
      <span class="na-st {a.status}" style="margin-top:4px">{a.status === "settled" ? "Settled" : a.status === "failed" ? "Failed" : "In-flight"}</span>
    </span>
  </a>
{/snippet}

<!-- ── Send modal ── -->
{#if modal === "send"}
  <div class="nw-scrim" onclick={() => (modal = null)} role="presentation"></div>
  <div class="nw-modal" role="dialog" aria-modal="true">
    {#if sendStep === "sending"}
      <div class="nw-mh"><span class="ttl">Sending</span></div>
      <div class="nw-mb"><div class="nw-done"><div class="nw-done-ic spin"><span class="wz-spinner" style="width:26px;height:26px;border-width:3px"></span></div><h3>Routing payment…</h3><div class="amt">Finding a path through the Lightning Network</div></div></div>
    {:else if sendStep === "done" && sentSummary}
      <div class="nw-mh"><span class="ttl">Sent</span><button class="nw-iconbtn" onclick={() => (modal = null)}><Icon name="close" size={16} /></button></div>
      <div class="nw-mb">
        <div class="nw-done">
          <div class="nw-done-ic"><Icon name="check" size={30} stroke={2.4} /></div>
          <h3>Payment sent</h3>
          <div class="amt">{sentSummary.amount ? commas(sentSummary.amount) + " sats" : "amount in payable"}</div>
          <div class="nw-proof"><div class="pk"><Icon name="shield" size={13} /> Payment id</div><div class="pv"><code>{sentSummary.id}</code><button class="wz-copybtn {copied === 'sid' ? 'copied' : ''}" onclick={() => copy(sentSummary!.id, "sid")}><Icon name={copied === "sid" ? "check" : "copy"} size={13} /></button></div></div>
        </div>
      </div>
      <div class="nw-mf"><button class="wz-btn" onclick={() => (modal = null)}>Done</button></div>
    {:else}
      <div class="nw-mh">
        <span class="ttl">{sendStep === 1 ? "Send bitcoin" : "Review & confirm"}</span>
        <button class="nw-iconbtn" onclick={() => (modal = null)}><Icon name="close" size={16} /></button>
      </div>
      <div class="nw-mb">
        {#if sendStep === 1}
          <span class="nw-flbl">Pay to</span>
          <textarea class="nw-input" bind:value={sendPayable} placeholder="Paste a Lightning invoice (lnbc…), BOLT12 offer (lno1…), Lightning address (you@domain), or bitcoin address (bc1…)"></textarea>
          <div style="height:14px"></div>
          <span class="nw-flbl">Amount (sats) — optional if the payable sets one</span>
          <input class="nw-input" inputmode="numeric" bind:value={sendAmount} placeholder="0" />
          <div style="height:14px"></div>
          <span class="nw-flbl">Note — optional (offers/LN-address only)</span>
          <input class="nw-input" bind:value={sendNote} placeholder="e.g. coffee" maxlength="200" />
          <p class="nw-hint">Sends from your Lightning balance (or on-chain for a bitcoin address). <b style="color:#FFB300">Bitcoin payments are irreversible</b> — check the destination.</p>
        {:else}
          <div class="nw-rev"><span class="k">To</span><span class="v">{sendPayable.trim().length > 36 ? sendPayable.trim().slice(0, 18) + "…" + sendPayable.trim().slice(-12) : sendPayable.trim()}</span></div>
          <div class="nw-rev"><span class="k">Amount</span><span class="v">{sendAmtSats ? commas(sendAmtSats) + " sats" : "set by payable"}</span></div>
          {#if sendNote.trim()}<div class="nw-rev"><span class="k">Note</span><span class="v">{sendNote.trim()}</span></div>{/if}
          {#if sendAmtSats && price > 0}<div class="nw-total"><span class="k">≈ USD</span><span class="v">{fmtUsd(usdOf(sendAmtSats))}</span></div>{/if}
          <p class="nw-hint" style="text-align:center;margin-top:14px">This moves real funds and cannot be reversed.</p>
          {#if sendErr}<p class="nw-err"><Icon name="warn" size={15} /> {sendErr}</p>{/if}
        {/if}
      </div>
      <div class="nw-mf">
        {#if sendStep === 1}
          <button class="wz-btn ghost" onclick={() => (modal = null)}>Cancel</button>
          <button class="wz-btn" disabled={!sendReady} onclick={() => (sendStep = 2)}>Review</button>
        {:else}
          <button class="wz-btn ghost" onclick={() => (sendStep = 1)}>Back</button>
          <button class="wz-btn" onclick={confirmSend}><Icon name="out" size={16} /> Confirm & send</button>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<!-- ── Receive modal ── -->
{#if modal === "receive"}
  <div class="nw-scrim" onclick={() => (modal = null)} role="presentation"></div>
  <div class="nw-modal" role="dialog" aria-modal="true">
    <div class="nw-mh"><span class="ttl">Receive</span><button class="nw-iconbtn" onclick={() => (modal = null)}><Icon name="close" size={16} /></button></div>
    <div class="nw-mb">
      <div class="nw-rtabs">
        {#each [["offer", "Offer"], ["invoice", "Invoice"]] as [k, l]}
          <button class={rcvTab === k ? "on" : ""} onclick={() => (rcvTab = k as typeof rcvTab)}>{l}</button>
        {/each}
      </div>
      {#if rcvTab === "invoice"}
        <span class="nw-flbl">Amount (sats) — optional</span>
        <input class="nw-input" inputmode="numeric" bind:value={rcvAmount} placeholder="0 (any amount)" />
        <div style="height:12px"></div>
        <button class="wz-btn subtle" style="width:100%;justify-content:center" disabled={rcvBusy} onclick={genInvoice}>
          {rcvBusy ? "Creating…" : rcvInvoice ? "Regenerate invoice" : "Create invoice"}
        </button>
        {#if rcvErr}<p class="nw-err"><Icon name="warn" size={15} /> {rcvErr}</p>{/if}
      {/if}
      <div style="height:14px"></div>
      <span class="nw-flbl">
        {rcvTab === "offer" ? "Reusable BOLT12 offer (registered with OCEAN)" : "Lightning invoice"}
      </span>
      <div class="nw-rval">
        <code>{rcvValue || (rcvTab === "invoice" ? "— create an invoice above —" : "— not available —")}</code>
        {#if rcvValue}
          <button class="wz-copybtn {copied === 'rcv' ? 'copied' : ''}" onclick={() => copy(rcvValue, "rcv")}><Icon name={copied === "rcv" ? "check" : "copy"} size={13} />{copied === "rcv" ? "Copied" : "Copy"}</button>
        {/if}
      </div>
      {#if rcvTab === "offer"}<p class="nw-hint">This is the offer you registered with OCEAN. Payouts to it are flagged as verified in your activity.</p>{/if}
    </div>
  </div>
{/if}
