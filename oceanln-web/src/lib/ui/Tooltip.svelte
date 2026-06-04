<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  let {
    label = "What's this?",
    enabled = true,
    children,
  }: { label?: string; enabled?: boolean; children?: Snippet } = $props();

  let open = $state(false);
  let el = $state<HTMLElement | undefined>(undefined);

  $effect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (el && !el.contains(e.target as Node)) open = false;
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  });
</script>

{#if enabled}
  <span class="wz-tip" bind:this={el}>
    <button class="wz-tip-btn" type="button" onclick={() => (open = !open)}>
      <Icon name="info" size={13} />{label}
    </button>
    {#if open}
      <span class="wz-tip-pop">{@render children?.()}</span>
    {/if}
  </span>
{/if}
