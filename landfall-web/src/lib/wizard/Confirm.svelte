<script lang="ts">
  import Icon from "../ui/Icon.svelte";
  import Callout from "../ui/Callout.svelte";
  import { app, CONFIRM_PICKS } from "../store.svelte";

  // Free-text entry, checked only when the user presses Continue (see
  // `continueStep`). A multiple-choice quiz with instant right/wrong colouring
  // could be clicked through in three tries per word without ever looking at
  // the backup; typed words with a single verdict cannot (QA-203).
  function set(qi: number, v: string) {
    app.answers = { ...app.answers, [qi]: v };
  }
</script>

<div class="wz-fade">
  <p class="wz-eyebrow"><Icon name="check" size={13} /> Quick check</p>
  <h1 class="wz-h">Confirm your backup</h1>
  <p class="wz-sub">
    Using only what you wrote down, type the word at each position below. All three are checked
    together when you press Continue.
  </p>

  {#each CONFIRM_PICKS as idx, qi}
    <div class="wz-confirm-q">
      <label class="q" for={`confirm-word-${idx + 1}`}>Word <b>#{idx + 1}</b></label>
      <input
        id={`confirm-word-${idx + 1}`}
        class="wz-input"
        style="max-width:280px"
        value={app.answers[qi] ?? ""}
        oninput={(e) => set(qi, (e.target as HTMLInputElement).value)}
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
      />
    </div>
  {/each}

  {#if app.error}
    <Callout kind="danger" icon="warn">{#snippet children()}{app.error}{/snippet}</Callout>
  {/if}
</div>
