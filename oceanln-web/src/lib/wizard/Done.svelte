<script lang="ts">
  import Icon from "../ui/Icon.svelte";
  import Tooltip from "../ui/Tooltip.svelte";
  import Callout from "../ui/Callout.svelte";
  import Button from "../ui/Button.svelte";
  import CopyField from "../ui/CopyField.svelte";
  import { app, guided, verifyOcean, go } from "../store.svelte";
</script>

{#if app.verifyState === "verified"}
  <div class="wz-fade">
    <div class="wz-seal"><Icon name="check" size={34} stroke={2} /></div>
    <p class="wz-eyebrow" style="color:#22c55e"><Icon name="bolt" size={13} /> Payouts enabled</p>
    <h1 class="wz-h">Lightning payouts are on</h1>
    <p class="wz-sub">
      You're all set. From your next payout onward, OCEAN will send your mining rewards straight
      to your Lightning wallet.
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
    <h1 class="wz-h">Send your details to OCEAN</h1>
    <p class="wz-sub">
      Here are the three things OCEAN needs. Submit them now, or copy each one into OCEAN's payout
      settings yourself — whichever you prefer.
    </p>

    <CopyField label="Payout address" chip="bc1q" value={app.miningAddress} />
    <CopyField label="Lightning offer" chip="BOLT12" value={app.offer} />
    <CopyField label="Signature" chip="BIP-322" value={app.signature} />

    <div class="wz-verify-row">
      {#if app.verifyState === "verifying"}
        <div class="wz-verifying"><span class="wz-spinner"></span> Verifying with OCEAN…</div>
      {:else}
        <Button icon="bolt" onclick={verifyOcean}>{#snippet children()}Verify &amp; turn on payouts{/snippet}</Button>
        <span style="font-size:12.5px;color:#52525b">or copy the fields above into OCEAN manually</span>
      {/if}
    </div>
  </div>
{/if}
