<script lang="ts">
  import Icon from "../ui/Icon.svelte";
  import Callout from "../ui/Callout.svelte";
  import Button from "../ui/Button.svelte";
  import { app, guided, isImport, generateWallet } from "../store.svelte";

  // Create mode: generate the phrase once when this step is first shown.
  $effect(() => {
    if (!isImport() && app.phrase.length === 0 && !app.busy && !app.error) {
      generateWallet();
    }
  });

  function fill(i: number, v: string) {
    app.importWords[i] = v.trim().toLowerCase();
  }
  function onPaste(e: ClipboardEvent) {
    const parts = (e.clipboardData?.getData("text") ?? "").trim().split(/\s+/);
    if (parts.length > 1) {
      e.preventDefault();
      parts.slice(0, app.importWords.length).forEach((w, i) => (app.importWords[i] = w.toLowerCase()));
    }
  }
</script>

{#if isImport()}
  <div class="wz-fade">
    <p class="wz-eyebrow"><Icon name="download" size={13} /> Restore wallet</p>
    <h1 class="wz-h">Enter your recovery phrase</h1>
    <p class="wz-sub">
      Type each of your 24 words in order, or paste the whole phrase into the first box.
    </p>
    <Callout kind="warn" icon="warn">
      {#snippet children()}
        Only enter your phrase on a device you trust. OCEAN staff will <b>never</b> ask you for
        these words.
      {/snippet}
    </Callout>
    <div class="wz-import">
      {#each app.importWords as w, i}
        <label class="ipt">
          <span class="n">{i + 1}</span>
          <input
            value={w}
            oninput={(e) => fill(i, (e.target as HTMLInputElement).value)}
            onpaste={i === 0 ? onPaste : undefined}
            autocapitalize="off"
            autocorrect="off"
            spellcheck="false"
          />
        </label>
      {/each}
    </div>
    {#if app.error}
      <Callout kind="danger" icon="warn">{#snippet children()}{app.error}{/snippet}</Callout>
    {/if}
  </div>
{:else}
  <div class="wz-fade">
    <p class="wz-eyebrow"><Icon name="key" size={13} /> Your recovery phrase</p>
    <h1 class="wz-h">Write down these 24 words</h1>
    <p class="wz-sub">
      This is the <strong>only</strong> backup of your wallet. Write the words on paper in this
      exact order and keep them somewhere safe and private. Whoever has these words controls the
      funds.
    </p>

    {#if app.error}
      <Callout kind="danger" icon="warn">
        {#snippet children()}
          Couldn't create your wallet: {app.error}
          <button class="wz-link" style="margin-left:8px" onclick={() => { app.error = ""; generateWallet(); }}>Retry</button>
        {/snippet}
      </Callout>
    {:else if app.phrase.length === 0}
      <div class="wz-verifying" style="padding:24px 0"><span class="wz-spinner"></span> Generating your recovery phrase…</div>
    {:else}
      <div class="wz-words-wrap" style="margin-bottom:18px">
        <div class="wz-words">
          {#each app.phrase as word, i}
            <div class="wz-word"><span class="wn">{i + 1}</span><span class="wt">{word}</span></div>
          {/each}
        </div>
        {#if !app.revealed}
          <div class="wz-blur" onclick={() => (app.revealed = true)} role="button" tabindex="0" onkeydown={(e) => e.key === "Enter" && (app.revealed = true)}>
            <div class="eye"><Icon name="eye" size={20} /></div>
            <div class="t">Tap to reveal your phrase</div>
            <div class="s">Make sure no one is looking over your shoulder</div>
          </div>
        {/if}
      </div>

      {#if app.revealed}
        <Callout kind="danger" icon="warn">
          {#snippet children()}
            <b>Never share these words and never take a screenshot.</b> Anyone who sees them can
            take your Bitcoin. No real service will ever ask for them.
          {/snippet}
        </Callout>
        <label class="wz-check">
          <input type="checkbox" checked={app.backedUp} onchange={(e) => (app.backedUp = (e.target as HTMLInputElement).checked)} />
          <span>I've written my recovery phrase down and stored it somewhere safe.</span>
        </label>
        {#if guided()}
          <p style="font-size:12px;color:#52525b;margin:14px 0 0;display:flex;align-items:center;gap:7px">
            <Icon name="lock" size={13} /> We don't keep a copy — there's no "forgot password" for this.
          </p>
        {/if}
      {/if}
    {/if}
  </div>
{/if}
