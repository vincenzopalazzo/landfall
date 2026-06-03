<script lang="ts">
  import Icon from "../ui/Icon.svelte";
  import { app, CONFIRM_PICKS } from "../store.svelte";
  import { WORD_POOL } from "../data";

  type Q = { idx: number; correct: string; opts: string[] };

  // Built once at mount; positions match CONFIRM_PICKS so gating in the store agrees.
  const quiz: Q[] = (() => {
    const decoys = WORD_POOL.filter((w) => !app.phrase.includes(w));
    return CONFIRM_PICKS.map((idx) => {
      const correct = app.phrase[idx];
      const opts = [correct];
      while (opts.length < 4) {
        const d = decoys[Math.floor(Math.random() * decoys.length)];
        if (!opts.includes(d)) opts.push(d);
      }
      for (let i = opts.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [opts[i], opts[j]] = [opts[j], opts[i]];
      }
      return { idx, correct, opts };
    });
  })();

  function pick(qi: number, word: string) {
    app.answers = { ...app.answers, [qi]: word };
  }
  function cls(qi: number, q: Q, w: string): string {
    const chosen = app.answers[qi];
    if (chosen === w) return w === q.correct ? "wz-opt correct" : "wz-opt wrong";
    return "wz-opt";
  }
</script>

<div class="wz-fade">
  <p class="wz-eyebrow"><Icon name="check" size={13} /> Quick check</p>
  <h1 class="wz-h">Confirm your backup</h1>
  <p class="wz-sub">
    Let's make sure your backup is correct. Using the words you just wrote down, tap the right
    word for each position below.
  </p>

  {#each quiz as q, qi}
    <div class="wz-confirm-q">
      <p class="q">Word <b>#{q.idx + 1}</b></p>
      <div class="wz-opts">
        {#each q.opts as w}
          <button class={cls(qi, q, w)} type="button" onclick={() => pick(qi, w)}>{w}</button>
        {/each}
      </div>
      {#if app.answers[qi] && app.answers[qi] !== q.correct}
        <p style="font-size:12px;color:#ef4444;margin:8px 0 0">
          That's not the right word — check your written backup and try again.
        </p>
      {/if}
    </div>
  {/each}
</div>
