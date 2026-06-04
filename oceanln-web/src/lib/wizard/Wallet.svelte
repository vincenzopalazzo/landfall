<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import Icon from "../ui/Icon.svelte";
  import Tooltip from "../ui/Tooltip.svelte";
  import Callout from "../ui/Callout.svelte";
  import Button from "../ui/Button.svelte";
  import CopyField from "../ui/CopyField.svelte";
  import { app, guided, createWalletAndOffer } from "../store.svelte";
  import { PROVISION_TASKS } from "../data";

  // No "describe" step: the offer description is derived from the payout address
  // (OCEAN Payouts for <addr>) inside createWalletAndOffer. This step just
  // provisions the wallet + creates the offer, automatically.
  let phase = $state<"running" | "done" | "error">(app.offer ? "done" : "running");
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
        phase = "error"; // app.error has the detail
      }
    });
  }

  function taskState(i: number): "done" | "active" | "" {
    if (phase === "done" || i < taskIndex) return "done";
    if (i === taskIndex) return "active";
    return "";
  }

  // Auto-start once on mount (skips the manual "describe + create" step).
  // onMount runs exactly once — no reactive re-entrancy.
  onMount(() => {
    if (!app.offer) start();
  });

  // Don't leak the cosmetic-progress interval if the user navigates away
  // (e.g. steps back) while provisioning is still in flight.
  onDestroy(() => clearInterval(timer));
</script>

{#if phase === "error"}
  <div class="wz-fade">
    <p class="wz-eyebrow"><Icon name="wallet" size={13} /> Create your wallet</p>
    <h1 class="wz-h">Couldn't create your wallet</h1>
    <p class="wz-sub">We hit a problem provisioning your node and creating the offer.</p>
    <Callout kind="danger" icon="warn">
      {#snippet children()}
        {app.error}. Check that oceanln-httpd can reach your Lexe node, then try again.
      {/snippet}
    </Callout>
    <Button icon="refresh" disabled={app.busy} onclick={start}>{#snippet children()}Try again{/snippet}</Button>
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
