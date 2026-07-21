<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { waitlist } from '$lib/waitlist.svelte';

  let { children } = $props();

  onMount(() => waitlist.load());
</script>

{@render children()}
