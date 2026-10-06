<script lang="ts">
  import Icon from "../ui/Icon.svelte";
  import Tooltip from "../ui/Tooltip.svelte";
  import Callout from "../ui/Callout.svelte";
  import Button from "../ui/Button.svelte";
  import CopyField from "../ui/CopyField.svelte";
  import { app, guided, canSign, signForOcean, resetSignature } from "../store.svelte";

  // The Sign button is gated on the pasted message embedding the offer.
  const offerMissing = $derived(app.oceanMessage.trim().length > 0 && !app.oceanMessage.includes(app.offer));
</script>

<div class="wz-fade">
  <p class="wz-eyebrow"><Icon name="pen" size={13} /> Prove it's you</p>
  <h1 class="wz-h">Sign OCEAN's verification message</h1>
  <p class="wz-sub">
    OCEAN's flow is offer-first: register your payout address and offer on ocean.xyz, OCEAN
    gives you a short <strong>verification message</strong>, and you sign it here to prove you
    control the address.{guided() ? " Signing costs nothing and moves no money." : ""}
  </p>

  <!-- Step 1: the details to register with OCEAN -->
  <p class="wz-section-label">1 · Give these to OCEAN</p>
  <CopyField label="Payout address" chip="bc1q" value={app.miningAddress}>
    {#snippet tip()}
      <Tooltip enabled={guided()}>
        {#snippet children()}
          Your Bitcoin payout address — this is your OCEAN username. Register it on ocean.xyz to
          get your verification message.
        {/snippet}
      </Tooltip>
    {/snippet}
  </CopyField>
  <CopyField label="Lightning offer" chip="BOLT12" value={app.offer} />

  <!-- Step 2: paste OCEAN's message and sign it -->
  <p class="wz-section-label" style="margin-top:22px">
    2 · Paste the message OCEAN gave you
    <Tooltip enabled={guided()} label=" what's this?">
      {#snippet children()}
        After you register, OCEAN shows a verification message that embeds your offer. Paste it
        here exactly — a <b>BIP-322 signature</b> over it proves you hold the address's key
        without revealing it, and can't move your funds.
      {/snippet}
    </Tooltip>
  </p>
  <textarea
    class="wz-input"
    style="min-height:96px;font-family:var(--font-mono);font-size:12.5px;line-height:1.5;resize:vertical"
    placeholder="Paste OCEAN's verification message here (it includes your lno1… offer)"
    bind:value={app.oceanMessage}
    disabled={!!app.signature}
  ></textarea>
  {#if offerMissing}
    <p style="font-size:12px;color:#FFB300;margin:8px 0 0;display:flex;gap:7px;align-items:center">
      <Icon name="warn" size={13} /> This message doesn't contain your offer — make sure you pasted the one OCEAN generated for this offer.
    </p>
  {/if}

  {#if app.error}
    <Callout kind="danger" icon="warn">{#snippet children()}Signing failed: {app.error}{/snippet}</Callout>
  {/if}

  {#if !app.signature}
    <div class="wz-verify-row">
      {#if app.busy}
        <div class="wz-verifying"><span class="wz-spinner"></span> Signing with your wallet…</div>
      {:else}
        <Button icon="pen" disabled={!canSign()} onclick={signForOcean}>{#snippet children()}Sign message{/snippet}</Button>
      {/if}
    </div>
  {:else}
    <div class="wz-fade" style="margin-top:6px">
      <Callout kind="ok" icon="check">
        {#snippet children()}<b>Signed.</b> Paste this signature back into OCEAN — it'll check it against your payout address.{/snippet}
      </Callout>
      <!-- What a BIP-322 signature actually commits to: this exact text, by the
           key behind this address. Shown verbatim so the user can see there is
           nothing hidden in what they are about to hand to OCEAN (QA-213). -->
      <p class="wz-section-label">You signed exactly this text</p>
      <pre
        data-testid="signed-message"
        style="margin:0 0 8px;padding:10px 12px;border:1px solid rgba(255,255,255,0.08);border-radius:8px;background:rgba(0,0,0,0.25);font-family:var(--font-mono);font-size:12px;line-height:1.5;white-space:pre-wrap;word-break:break-all"
      >{app.message}</pre>
      <p style="font-size:12px;color:#a1a1aa;margin:0 0 16px;line-height:1.6">
        with the key behind <code data-testid="signed-address">{app.miningAddress}</code>. OCEAN checks the
        signature below against that address and this exact text — nothing else is signed, and the
        signature cannot move funds.
      </p>
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
      <div class="wz-verify-row">
        <Button variant="ghost" icon="refresh" onclick={resetSignature}>{#snippet children()}Sign a different message{/snippet}</Button>
      </div>
    </div>
  {/if}
</div>
