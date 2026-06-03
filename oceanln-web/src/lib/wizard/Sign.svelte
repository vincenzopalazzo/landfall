<script lang="ts">
  import Icon from "../ui/Icon.svelte";
  import Tooltip from "../ui/Tooltip.svelte";
  import Callout from "../ui/Callout.svelte";
  import Button from "../ui/Button.svelte";
  import CopyField from "../ui/CopyField.svelte";
  import { app, guided, signForOcean } from "../store.svelte";
</script>

<div class="wz-fade">
  <p class="wz-eyebrow"><Icon name="pen" size={13} /> Prove it's you</p>
  <h1 class="wz-h">Sign OCEAN's verification message</h1>
  <p class="wz-sub">
    To switch on payouts, OCEAN needs proof you control your payout address. You'll sign a short
    message with your wallet — like a digital signature.{guided() ? " It costs nothing and moves no money." : ""}
  </p>

  <p class="wz-section-label">
    Message to sign
    <Tooltip enabled={guided()} label=" what's a signature?">
      {#snippet children()}
        A <b>BIP-322 signature</b> proves you hold the private key for your address without
        revealing it. It's math, not a password — and it can't be used to spend your funds.
      {/snippet}
    </Tooltip>
  </p>
  <div class="wz-copy" style="margin-bottom:18px">
    <div class="val" style="white-space:pre-wrap;color:#a1a1aa">
      {#if app.message}{app.message}{:else}OCEAN Lightning Payout Authorization
Pool: ocean.xyz
Payout address: {app.miningAddress}
Offer: {app.offer}
…issued + nonce added when you sign{/if}
    </div>
  </div>

  {#if app.error}
    <Callout kind="danger" icon="warn">{#snippet children()}Signing failed: {app.error}{/snippet}</Callout>
  {/if}

  {#if !app.signature}
    <div class="wz-verify-row">
      {#if app.busy}
        <div class="wz-verifying"><span class="wz-spinner"></span> Signing with your wallet…</div>
      {:else}
        <Button icon="pen" onclick={signForOcean}>{#snippet children()}Sign message{/snippet}</Button>
      {/if}
    </div>
  {:else}
    <div class="wz-fade">
      <Callout kind="ok" icon="check">
        {#snippet children()}<b>Signed.</b> Here's your signature — OCEAN will check it against your payout address.{/snippet}
      </Callout>
      <CopyField label="Your signature" chip="BIP-322" value={app.signature}>
        {#snippet tip()}
          <Tooltip enabled={guided()}>
            {#snippet children()}
              This string is the proof. OCEAN verifies it matches your address; it can't be reused
              to access your wallet.
            {/snippet}
          </Tooltip>
        {/snippet}
      </CopyField>
    </div>
  {/if}
</div>
