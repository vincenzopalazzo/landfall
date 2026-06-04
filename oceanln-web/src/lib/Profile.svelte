<script lang="ts">
  import Icon from "./ui/Icon.svelte";
  import Callout from "./ui/Callout.svelte";
  import { app, go, restart, client } from "./store.svelte";

  // profile is seeded before navigating here.
  const profile = $derived(app.profile!);
  let revealed = $state(false);
  let addingOffer = $state(false);

  function deriveAddress() {
    const n = profile.addresses.length + 1;
    // No "derive at index N" endpoint yet — this row is a placeholder until a
    // payout derives it. Linking still works.
    profile.addresses = [
      ...profile.addresses,
      { id: "a" + n, label: "Payout address " + n, address: "— derived on next payout —", offerId: profile.offers[0]?.id ?? null },
    ];
  }
  async function newOffer() {
    addingOffer = true;
    app.error = "";
    try {
      const r = await client().offer("Offer " + (profile.offers.length + 1));
      profile.offers = [
        ...profile.offers,
        { id: "o" + (profile.offers.length + 1), label: "Offer " + (profile.offers.length + 1), value: r.offer },
      ];
    } catch (e) {
      app.error = e instanceof Error ? e.message : String(e);
    } finally {
      addingOffer = false;
    }
  }
  function linkCount(offerId: string): number {
    return profile.addresses.filter((a) => a.offerId === offerId).length;
  }
</script>

<div class="app-scroll">
  <div class="pf-head">
    <div class="pf-av">O</div>
    <div>
      <h1 class="pf-name">My OCEAN wallet</h1>
      <p class="pf-id">
        {app.miningAddress.slice(0, 14)}…{app.miningAddress.slice(-6)}
        <!-- We can't claim "Verified": OCEAN confirms the signature on its side,
             and there's no API here to read that back. Reflect only what we know
             locally — whether the user has submitted their details to OCEAN. -->
        {#if app.submitted}
          <span class="db-chip muted"><span class="db-dot"></span>Submitted to OCEAN</span>
        {/if}
      </p>
    </div>
  </div>

  {#if app.error}
    <Callout kind="danger" icon="warn">{#snippet children()}{app.error}{/snippet}</Callout>
  {/if}

  <!-- Onchain payout addresses -->
  <div class="pf-section">
    <div class="pf-section-h">
      <h2><Icon name="key" size={15} /> Onchain payout addresses</h2>
      <button class="pf-add" onclick={deriveAddress}><Icon name="key" size={13} /> Derive address</button>
    </div>
    <div class="pf-list">
      {#each profile.addresses as addr}
        <div class="pf-addr">
          <div class="ic"><Icon name="key" size={16} /></div>
          <div class="meta">
            <div class="lbl">{addr.label}</div>
            <div class="val">{addr.address}</div>
          </div>
          <div class="link">
            <span class="pf-link-lbl">Linked offer</span>
            <select class="pf-select" bind:value={addr.offerId}>
              <option value={null}>Not linked</option>
              {#each profile.offers as o}
                <option value={o.id}>{o.label}</option>
              {/each}
            </select>
          </div>
        </div>
      {/each}
    </div>
  </div>

  <!-- Lightning offers -->
  <div class="pf-section">
    <div class="pf-section-h">
      <h2><Icon name="bolt" size={15} /> Lightning offers</h2>
      <button class="pf-add" onclick={newOffer} disabled={addingOffer}>
        <Icon name="spark" size={13} /> {addingOffer ? "Creating…" : "New offer"}
      </button>
    </div>
    <div class="pf-list">
      {#each profile.offers as o}
        <div class="pf-offerrow">
          <div class="ic"><Icon name="wallet" size={16} /></div>
          <div class="meta">
            <div class="lbl">{o.label}</div>
            <div class="val">{o.value}</div>
          </div>
          <span class="count">{linkCount(o.id)} address{linkCount(o.id) === 1 ? "" : "es"}</span>
        </div>
      {/each}
    </div>
  </div>

  <!-- Recovery phrase -->
  <div class="pf-section">
    <div class="pf-section-h"><h2><Icon name="lock" size={15} /> Recovery phrase</h2></div>
    <div class="pf-phrase">
      <div class="ph-top">
        <span class="t"><Icon name="key" size={15} /> Your 24 words</span>
        <button class="wz-copybtn" onclick={() => (revealed = !revealed)}>
          <Icon name="eye" size={13} /> {revealed ? "Hide" : "Reveal"}
        </button>
      </div>
      {#if revealed && app.phrase.length}
        <div class="wz-words">
          {#each app.phrase as word, i}
            <div class="wz-word"><span class="wn">{i + 1}</span><span class="wt">{word}</span></div>
          {/each}
        </div>
        <p style="font-size:12px;color:#52525b;margin:12px 0 0;display:flex;gap:7px;align-items:center"><Icon name="warn" size={13} /> Never share these words or take a screenshot.</p>
      {:else if revealed}
        <p style="font-size:13px;color:#71717a;margin:0">Phrase isn't held in this session — re-run setup or import to view it.</p>
      {:else}
        <p style="font-size:13px;color:#71717a;margin:0">Hidden. Reveal only when you're somewhere private.</p>
      {/if}
    </div>
  </div>

  <!-- Shortcuts -->
  <div class="pf-tiles">
    <button class="pf-tile" onclick={() => go("dashboard")}>
      <span class="tic"><Icon name="bolt" size={19} /></span>
      <div><h3>Lightning dashboard</h3><p>Monitor payouts, node status, and your offer.</p></div>
      <span class="arr"><Icon name="arrowR" size={16} /></span>
    </button>
    <button class="pf-tile" onclick={restart}>
      <span class="tic"><Icon name="refresh" size={19} /></span>
      <div><h3>Re-run setup</h3><p>Start the onboarding wizard again.</p></div>
      <span class="arr"><Icon name="arrowR" size={16} /></span>
    </button>
  </div>
</div>
