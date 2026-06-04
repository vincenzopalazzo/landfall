<script lang="ts">
  import Icon from "../ui/Icon.svelte";
  import Tooltip from "../ui/Tooltip.svelte";
  import Callout from "../ui/Callout.svelte";
  import { chooseMode, guided } from "../store.svelte";
</script>

<div class="wz-fade">
  <p class="wz-eyebrow"><Icon name="bolt" size={13} /> Lightning payouts</p>
  <h1 class="wz-h">Get paid your mining rewards over Lightning</h1>
  <p class="wz-sub">
    Instead of waiting for on-chain payouts, OCEAN can send your rewards to a Lightning
    wallet you fully control. This setup takes about three minutes — we'll walk you through
    every step.
  </p>

  <div class="wz-card" style="margin-bottom:22px;display:flex;gap:16px;align-items:flex-start">
    <div style="width:38px;height:38px;border-radius:10px;flex-shrink:0;background:var(--wiz-accent-dim);color:var(--wiz-accent);display:flex;align-items:center;justify-content:center">
      <Icon name="key" size={19} />
    </div>
    <div>
      <div style="font-size:14px;font-weight:600;color:#fafafa;margin-bottom:4px;display:flex;align-items:center;gap:8px">
        One recovery phrase runs everything
        <Tooltip enabled={guided()}>
          {#snippet children()}
            <b>Your recovery phrase</b> is a list of words that is the master key to your money.
            Here, the same phrase powers both your Lightning wallet and the Bitcoin address OCEAN
            pays you at — so there's only one thing to back up.
          {/snippet}
        </Tooltip>
      </div>
      <p style="font-size:13px;line-height:1.6;color:#a1a1aa;margin:0">
        A single set of backup words controls your Lightning wallet <em>and</em> your payout
        address. Back it up once and you're covered for both.
      </p>
    </div>
  </div>

  <Callout kind="info" icon="shield">
    {#snippet children()}
      <b>You stay in control.</b> Your wallet runs in a private, sealed environment (powered by
      Lexe) that only you can open. OCEAN never holds your keys or your funds — it only learns
      where to send your rewards.
    {/snippet}
  </Callout>

  <p class="wz-section-label" style="margin-top:26px">How would you like to start?</p>
  <div class="wz-choices">
    <button class="wz-choice" onclick={() => chooseMode("create")}>
      <span class="rec">Recommended</span>
      <span class="ic"><Icon name="spark" size={20} /></span>
      <h4>Create a new wallet</h4>
      <p>We'll generate a fresh recovery phrase and set everything up for you.</p>
    </button>
    <button class="wz-choice" onclick={() => chooseMode("import")}>
      <span class="ic"><Icon name="download" size={20} /></span>
      <h4>I already have a phrase</h4>
      <p>Restore an existing 24-word recovery phrase you've used before.</p>
    </button>
  </div>
</div>
