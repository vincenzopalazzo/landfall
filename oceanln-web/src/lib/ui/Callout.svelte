<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  let {
    kind = "info",
    icon = "info",
    children,
  }: { kind?: "info" | "warn" | "danger" | "ok"; icon?: string; children?: Snippet } = $props();
</script>

<div class="wz-callout {kind}">
  <Icon name={icon} size={17} />
  <div class="ct">{@render children?.()}</div>
</div>
