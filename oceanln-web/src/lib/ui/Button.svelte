<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  let {
    variant = "primary",
    icon,
    iconRight,
    disabled = false,
    onclick,
    children,
  }: {
    variant?: "primary" | "ghost" | "subtle";
    icon?: string;
    iconRight?: string;
    disabled?: boolean;
    onclick?: () => void;
    children?: Snippet;
  } = $props();

  const cls = $derived(
    variant === "primary" ? "wz-btn" : variant === "ghost" ? "wz-btn ghost" : "wz-btn subtle",
  );
</script>

<button class={cls} type="button" {disabled} {onclick}>
  {#if icon}<Icon name={icon} size={16} />{/if}
  {@render children?.()}
  {#if iconRight}<Icon name={iconRight} size={16} />{/if}
</button>
