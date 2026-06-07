<script lang="ts">
  import Icon from "./lib/ui/Icon.svelte";
  import Button from "./lib/ui/Button.svelte";
  import Settings from "./lib/Settings.svelte";
  import Welcome from "./lib/wizard/Welcome.svelte";
  import Phrase from "./lib/wizard/Phrase.svelte";
  import Confirm from "./lib/wizard/Confirm.svelte";
  import Wallet from "./lib/wizard/Wallet.svelte";
  import Sign from "./lib/wizard/Sign.svelte";
  import Done from "./lib/wizard/Done.svelte";
  import Profile from "./lib/Profile.svelte";
  import Dashboard from "./lib/Dashboard.svelte";
  import {
    app, STEPS, stepKey, canContinue, continueStep, goBack, railClick,
    stepState, restart, go, visibleSteps, humanIndex, refreshHealth, bootstrap,
  } from "./lib/store.svelte";

  const ACCENTS = {
    orange: { c: "#f7931a", dim: "rgba(247,147,26,0.10)", line: "rgba(247,147,26,0.28)" },
    blue: { c: "#4D6BFF", dim: "rgba(77,107,255,0.12)", line: "rgba(77,107,255,0.32)" },
  };

  // Apply the accent globally whenever it changes.
  $effect(() => {
    const a = ACCENTS[app.accent];
    const r = document.documentElement.style;
    r.setProperty("--wiz-accent", a.c);
    r.setProperty("--wiz-accent-dim", a.dim);
    r.setProperty("--wiz-accent-line", a.line);
  });

  // Health check on load + whenever the base URL changes.
  $effect(() => {
    void app.base;
    refreshHealth();
  });

  // Bootstrap on launch AND whenever the credentials change. In the browser
  // build, `app.token` starts empty in production — the first bootstrap will
  // 401 and silently leave the user in onboarding. When they paste the
  // bearer token (or change the base URL) in Settings, re-run bootstrap so
  // an already-configured wallet is detected and the user lands on the
  // restored profile/offer instead of being asked to generate a new one.
  //
  // Tauri build: `app.token` is unused (in-process IPC has no token) and
  // doesn't change at runtime, so the effect re-runs once on mount only.
  $effect(() => {
    void app.token;
    void app.base;
    void bootstrap();
  });
</script>

<!-- Full-frame app surface (no OS window chrome); branding lives in the rail / top nav. -->
<div class="wz-window">
  <div class="wz-body">
    {#if app.surface === "profile" || app.surface === "dashboard"}
      <div style="flex:1;display:flex;flex-direction:column;min-width:0">
        <div class="app-nav">
          <div class="brand"><img src="/assets/OCEAN-logo-white.svg" alt="OCEAN" /></div>
          <button class="pill {app.surface === 'profile' ? 'active' : ''}" onclick={() => go("profile")}><Icon name="key" size={14} /> Profile</button>
          <button class="pill {app.surface === 'dashboard' ? 'active' : ''}" onclick={() => go("dashboard")}><Icon name="bolt" size={14} /> Lightning</button>
          <span class="spacer"></span>
          <span class="acct"><span class="av">O</span> My wallet</span>
        </div>
        {#if app.surface === "profile"}<Profile />{:else}<Dashboard />{/if}
      </div>
    {:else}
      <!-- rail -->
      <div class="wz-rail">
        <div class="wz-rail-brand"><img src="/assets/OCEAN-logo-white.svg" alt="OCEAN" /><span class="tag">Lightning</span></div>
        {#each STEPS as s, i}
          {@const st = stepState(i)}
          <div
            class="wz-step {st === 'skip' ? '' : st}"
            style="cursor:{i < app.stepIndex && st !== 'skip' ? 'pointer' : 'default'};opacity:{st === 'skip' ? 0.32 : 1}"
            onclick={() => railClick(i)}
            onkeydown={(e) => e.key === "Enter" && railClick(i)}
            role="button"
            tabindex="-1"
          >
            <div class="num">
              {#if st === "done"}<Icon name="check" size={13} stroke={2} />
              {:else if st === "skip"}–
              {:else}{i + 1}{/if}
            </div>
            <div class="lbl">{s.label}</div>
          </div>
        {/each}
        <div class="wz-rail-foot">
          <div class="note"><Icon name="lock" size={13} /><span>Your keys and recovery phrase stay on your device.</span></div>
        </div>
      </div>

      <!-- content + footer -->
      <div class="wz-content">
        <div class="wz-scroll">
          {#key app.stepIndex}
            {#if stepKey() === "welcome"}<Welcome />
            {:else if stepKey() === "phrase"}<Phrase />
            {:else if stepKey() === "confirm"}<Confirm />
            {:else if stepKey() === "wallet"}<Wallet />
            {:else if stepKey() === "sign"}<Sign />
            {:else}<Done />{/if}
          {/key}
        </div>
        <div class="wz-foot">
          {#if app.stepIndex > 0}
            <Button variant="ghost" icon="arrowL" onclick={goBack}>{#snippet children()}Back{/snippet}</Button>
          {/if}
          <span class="spacer"></span>
          <span class="wz-progress-text">Step {Math.min(humanIndex(), visibleSteps())} of {visibleSteps()}</span>
          {#if stepKey() === "done"}
            {#if app.submitted}
              <Button icon="refresh" onclick={restart}>{#snippet children()}Start over{/snippet}</Button>
            {/if}
          {:else if stepKey() !== "welcome"}
            <Button iconRight="arrowR" disabled={!canContinue() || app.busy} onclick={continueStep}>{#snippet children()}Continue{/snippet}</Button>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<Settings />
