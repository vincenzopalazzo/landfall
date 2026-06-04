<script lang="ts">
  import Icon from "../ui/Icon.svelte";
  import Tooltip from "../ui/Tooltip.svelte";
  import Callout from "../ui/Callout.svelte";
  import Button from "../ui/Button.svelte";
  import CopyField from "../ui/CopyField.svelte";
  import { app, guided, createWalletAndOffer } from "../store.svelte";
  import { PROVISION_TASKS, OFFER_SUGGESTIONS } from "../data";

  let phase = $state<"input" | "running" | "done">(app.offer ? "done" : "input");
  let taskIndex = $state(app.offer ? PROVISION_TASKS.length : 0);
  let timer: ReturnType<typeof setInterval> | undefined;

  function start() {
    phase = "running";
    taskIndex = 0;
    // Cosmetic progress: step through tasks but never reach "all done" until the
    // real /init + /offer calls resolve.
    timer = setInterval(() => {
      if (taskIndex < PROVISION_TASKS.length - 1) taskIndex += 1;
    }, 900);
    createWalletAndOffer().then((ok) => {
      clearInterval(timer);
      if (ok) {
        taskIndex = PROVISION_TASKS.length;
        phase = "done";
      } else {
        phase = "input"; // error is shown via app.error
      }
    });
  }

  function taskState(i: number): "done" | "active" | "" {
    if (phase === "done" || i < taskIndex) return "done";
    if (i === taskIndex) return "active";
    return "";
  }
</script>

{#if phase === "input"}
  <div class="wz-fade">
    <p class="wz-eyebrow"><Icon name="wallet" size={13} /> Create your wallet</p>
    <h1 class="wz-h">Describe your Lightning offer</h1>
    <p class="wz-sub">
      Add a short description so you (and OCEAN) can recognize this payout destination. It's
      saved inside your <strong>BOLT12 offer</strong> and shown on every payment.
    </p>
    {#if app.miningAddress}
      <CopyField label="Your payout address" chip="bc1q" value={app.miningAddress}>
        {#snippet tip()}
          <Tooltip enabled={guided()}>
            {#snippet children()}
              Derived from your recovery phrase — this is the address OCEAN pays out to. The offer
              you're about to create is for this wallet, so you can reference it in the description.
            {/snippet}
          </Tooltip>
        {/snippet}
      </CopyField>
    {/if}
    <div class="wz-field">
      <label>
        Offer description
        <Tooltip enabled={guided()}>
          {#snippet children()}
            This label is encoded into your <b>BOLT12 offer</b>. It travels with the offer so any
            payment to it carries this note — handy for bookkeeping.
          {/snippet}
        </Tooltip>
      </label>
      <input
        class="wz-input"
        bind:value={app.offerDescription}
        maxlength="64"
        placeholder="e.g. OCEAN mining payouts"
      />
      <div class="wz-suggest">
        {#each OFFER_SUGGESTIONS as s}
          <button class="wz-chip-btn" type="button" onclick={() => (app.offerDescription = s)}>{s}</button>
        {/each}
      </div>
    </div>
    {#if app.error}
      <Callout kind="danger" icon="warn">
        {#snippet children()}
          Couldn't create your wallet: {app.error}. Check that oceanln-httpd can reach your Lexe
          node, then try again.
        {/snippet}
      </Callout>
    {:else}
      <Callout kind="info" icon="enclave">
        {#snippet children()}
          When you continue, we start your node in a sealed enclave and generate your offer and
          payout address — this happens once and takes a few seconds.
        {/snippet}
      </Callout>
    {/if}
    <Button icon="spark" disabled={!app.offerDescription.trim() || app.busy} onclick={start}>
      {#snippet children()}Create wallet &amp; offer{/snippet}
    </Button>
  </div>
{:else}
  <div class="wz-fade">
    <p class="wz-eyebrow"><Icon name="wallet" size={13} /> {phase === "done" ? "Wallet ready" : "Creating your wallet"}</p>
    <h1 class="wz-h">{phase === "done" ? "Your wallet is ready" : "Setting up your Lightning wallet"}</h1>
    <p class="wz-sub">
      {phase === "done"
        ? "Your Lightning wallet is live and we've generated your payout details from your recovery phrase."
        : "Hang tight — this only happens once. We're starting your node and building your offer and payout address."}
    </p>

    <div class="wz-tasks">
      {#each PROVISION_TASKS as tk, i}
        {@const st = taskState(i)}
        <div class="wz-task {st}">
          <div class="tk-ic">
            {#if st === "done"}<Icon name="check" size={14} />
            {:else if st === "active"}<span class="wz-spinner"></span>
            {:else}<span style="font-family:var(--font-mono);font-size:11px">{i + 1}</span>{/if}
          </div>
          <div>
            <div class="tk-t">{tk.t}</div>
            {#if st === "active" || (guided() && st === "done")}<div class="tk-s">{tk.s}</div>{/if}
          </div>
        </div>
      {/each}
    </div>

    {#if phase === "done"}
      <div class="wz-fade" style="margin-top:22px">
        <Callout kind="ok" icon="enclave">
          {#snippet children()}
            <b>Running in a sealed enclave.</b> Your node lives in hardware only you can unlock.
            Even Lexe can't see your keys or move your funds.
          {/snippet}
        </Callout>
        <CopyField label="Your Lightning address (offer)" chip="BOLT12" value={app.offer}>
          {#snippet tip()}
            <Tooltip enabled={guided()}>
              {#snippet children()}
                <b>A BOLT12 offer</b> is a reusable Lightning address (it starts with <b>lno1</b>).
                OCEAN sends your rewards to it — you can reuse it forever.
              {/snippet}
            </Tooltip>
          {/snippet}
        </CopyField>
        {#if app.offerDescription.trim()}
          <div class="wz-offer-desc"><Icon name="info" size={14} /> Description encoded in offer: <b>{app.offerDescription}</b></div>
        {/if}
        <CopyField label="Your payout address" chip="bc1q" value={app.miningAddress} />
      </div>
    {/if}
  </div>
{/if}
