<script lang="ts">
  import Icon from "../ui/Icon.svelte";
  import Callout from "../ui/Callout.svelte";
  import Button from "../ui/Button.svelte";
  import CopyField from "../ui/CopyField.svelte";
  import { app, markSubmittedToOcean, go } from "../store.svelte";
</script>

{#if app.submitted}
  <div class="wz-fade">
    <div class="wz-seal"><Icon name="check" size={34} stroke={2} /></div>
    <p class="wz-eyebrow" style="color:#22c55e"><Icon name="bolt" size={13} /> Setup complete</p>
    <h1 class="wz-h">You're ready for Lightning payouts</h1>
    <p class="wz-sub">
      Once OCEAN verifies your signature, it will send your mining rewards straight to your
      Lightning wallet from the next payout onward. Verification happens on OCEAN's side — you
      can check its status in your payout settings on ocean.xyz.
    </p>
    <div class="wz-summary">
      <div class="row"><span class="ri"><Icon name="wallet" size={17} /></span><span class="rk">Lightning offer</span><span class="rv">{app.offer}</span></div>
      <div class="row"><span class="ri"><Icon name="key" size={17} /></span><span class="rk">Payout address</span><span class="rv">{app.miningAddress}</span></div>
      <div class="row"><span class="ri"><Icon name="shield" size={17} /></span><span class="rk">Self-custody</span><span class="rv">You hold the keys</span></div>
    </div>
    <Callout kind="info" icon="info">
      {#snippet children()}Keep your recovery phrase safe. It's the only way to restore this wallet and your payouts.{/snippet}
    </Callout>
    <div style="margin-top:22px">
      <Button icon="wallet" onclick={() => go("profile")}>{#snippet children()}Go to my profile{/snippet}</Button>
    </div>
  </div>
{:else}
  <div class="wz-fade">
    <p class="wz-eyebrow"><Icon name="link" size={13} /> Final step</p>
    <h1 class="wz-h">Submit your details to OCEAN</h1>
    <p class="wz-sub">
      Paste these three values into your payout settings on ocean.xyz. OCEAN checks the signature
      against your address, then routes your rewards to your Lightning offer. Nothing is sent from
      here — you complete the hand-off on OCEAN's site.
    </p>

    <CopyField label="Payout address" chip="bc1q" value={app.miningAddress} />
    <CopyField label="Lightning offer" chip="BOLT12" value={app.offer} />
    <CopyField label="Signature" chip="BIP-322" value={app.signature} />

    <div class="wz-verify-row" style="flex-wrap:wrap;gap:12px">
      <a
        class="wz-btn"
        href="https://ocean.xyz"
        target="_blank"
        rel="noreferrer"
        style="text-decoration:none;display:inline-flex;align-items:center;gap:8px"
      >
        <Icon name="link" size={15} /> Open OCEAN payout settings
      </a>
      <Button icon="check" variant="ghost" onclick={markSubmittedToOcean}>
        {#snippet children()}I've submitted these to OCEAN{/snippet}
      </Button>
    </div>
  </div>
{/if}
