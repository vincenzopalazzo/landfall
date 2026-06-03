<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  let {
    label,
    chip = "",
    value,
    mono = true,
    tip,
  }: { label: string; chip?: string; value: string; mono?: boolean; tip?: Snippet } = $props();

  let copied = $state(false);
  function copy() {
    navigator.clipboard?.writeText(value).catch(() => {});
    copied = true;
    setTimeout(() => (copied = false), 1600);
  }
</script>

<div class="wz-copy">
  <div class="top">
    <span class="lbl">
      {label}
      {#if chip}<span class="chip">{chip}</span>{/if}
      {#if tip}{@render tip()}{/if}
    </span>
    <button class="wz-copybtn {copied ? 'copied' : ''}" type="button" onclick={copy}>
      <Icon name={copied ? "check" : "copy"} size={13} />{copied ? "Copied" : "Copy"}
    </button>
  </div>
  <div class="val {mono ? '' : 'muted'}">{value}</div>
</div>
