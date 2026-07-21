<script lang="ts">
  import { isEmail, waitlist } from '$lib/waitlist.svelte';

  let { variant = 'hero' }: { variant?: 'hero' | 'cta' } = $props();

  let email = $state('');
  let error = $state('');
  let submitted = $state(false);

  function submit(event: SubmitEvent) {
    event.preventDefault();
    const value = email.trim();
    if (!isEmail(value)) {
      error = variant === 'hero' ? 'Enter a valid email address.' : '';
      return;
    }
    error = '';
    waitlist.add(value);
    submitted = true;
  }
</script>

{#if variant === 'hero'}
  <form class="hero-form" class:hide={submitted} onsubmit={submit} novalidate>
    <input
      type="email"
      bind:value={email}
      placeholder="you@domain.com"
      autocomplete="email"
      aria-label="Email address"
    />
    <button type="submit" class="btn btn-primary">Request access <span class="arrow">&#8594;</span></button>
  </form>
  <div class="hero-err" role="alert">{error}</div>
  <div class="form-ok" class:show={submitted}>
    <span class="material-symbols-sharp">check_circle</span>
    <span>You're on the list. We'll email <b>{waitlist.lastEmail}</b> when your spot opens.</span>
  </div>
{:else}
  <form class="cta-form" class:hide={submitted} onsubmit={submit} novalidate>
    <input
      type="email"
      bind:value={email}
      placeholder="you@domain.com"
      autocomplete="email"
      aria-label="Email address"
    />
    <button type="submit" class="btn btn-navy">Request access <span class="arrow">&#8594;</span></button>
  </form>
  <div class="cta-ok" class:show={submitted}>
    <span class="material-symbols-sharp">check_circle</span><span>You're on the list — invite incoming.</span>
  </div>
{/if}

<style>
  /* ── Hero variant ── */
  .hero-form {
    display: flex;
    gap: 10px;
    max-width: 480px;
  }
  .hero-form input {
    flex: 1;
    min-width: 0;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.28);
    border-radius: var(--radius-pill);
    padding: 14px 20px;
    color: #fff;
    font-family: var(--font-sans);
    font-size: 15px;
    outline: none;
    transition:
      border-color var(--dur-base),
      background var(--dur-base);
  }
  .hero-form input::placeholder {
    color: rgba(255, 255, 255, 0.6);
  }
  .hero-form input:focus {
    border-color: #06ffd2;
    background: rgba(255, 255, 255, 0.14);
  }
  .hero-form :global(.btn-primary) {
    background: #fff;
    color: var(--ocean-blue);
  }
  .hero-form :global(.btn-primary:hover) {
    background: #06ffd2;
    color: var(--ocean-navy);
    box-shadow: 0 4px 24px rgba(6, 255, 210, 0.4);
  }
  .hero-err {
    color: #ffd1d1;
    font-size: 13px;
    margin-top: 10px;
    min-height: 16px;
  }
  .form-ok {
    display: none;
    align-items: center;
    gap: 11px;
    background: rgba(6, 255, 210, 0.12);
    border: 1px solid rgba(6, 255, 210, 0.4);
    border-radius: 16px;
    padding: 16px 20px;
    max-width: 480px;
    color: #fff;
    font-size: 15px;
    line-height: 1.4;
  }
  .form-ok .material-symbols-sharp {
    font-size: 22px;
    color: #06ffd2;
    flex-shrink: 0;
  }
  .form-ok.show {
    display: flex;
  }

  /* ── CTA variant ── */
  .cta-form {
    display: flex;
    gap: 10px;
    max-width: 470px;
    margin: 0 auto;
  }
  .cta-form input {
    flex: 1;
    min-width: 0;
    background: rgba(2, 9, 45, 0.06);
    border: 1px solid rgba(2, 9, 45, 0.2);
    border-radius: var(--radius-pill);
    padding: 14px 20px;
    color: var(--ocean-navy);
    font-family: var(--font-sans);
    font-size: 15px;
    outline: none;
  }
  .cta-form input::placeholder {
    color: rgba(2, 9, 45, 0.5);
  }
  .cta-form input:focus {
    border-color: var(--ocean-navy);
  }
  .cta-ok {
    display: none;
    align-items: center;
    justify-content: center;
    gap: 11px;
    font-size: 16px;
    font-weight: 600;
    color: var(--ocean-navy);
  }
  .cta-ok .material-symbols-sharp {
    font-size: 24px;
  }
  .cta-ok.show {
    display: flex;
  }

  .hide {
    display: none;
  }

  @media (max-width: 560px) {
    .hero-form,
    .cta-form {
      flex-direction: column;
    }
  }
</style>
