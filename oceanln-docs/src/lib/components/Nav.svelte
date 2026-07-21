<script lang="ts">
  import { base } from '$app/paths';

  type NavLink = { href: string; label: string };

  let {
    links = [
      { href: `${base}/#payouts`, label: 'Why Lightning' },
      { href: `${base}/#how`, label: 'How it works' },
      { href: `${base}/docs`, label: 'Docs' }
    ],
    cta = { href: `${base}/#access`, label: 'Request access' }
  }: { links?: NavLink[]; cta?: NavLink } = $props();
</script>

<nav class="nav">
  <div class="nav-in">
    <a href="{base}/" class="nav-home" aria-label="OCEAN home">
      <img class="nav-logo" src="{base}/assets/OCEAN-logo-dark.svg" alt="OCEAN" />
    </a>
    <div class="nav-links">
      {#each links as link}
        <a href={link.href}>{link.label}</a>
      {/each}
    </div>
    <div class="nav-cta">
      <a href="https://ocean.xyz" class="ghost" target="_blank" rel="noreferrer noopener"
        >ocean.xyz &nbsp;&#8599;</a
      >
      <a href={cta.href} class="btn btn-primary btn-sm">{cta.label} <span class="arrow">&#8594;</span></a>
    </div>
  </div>
</nav>

<style>
  .nav {
    position: sticky;
    top: 0;
    z-index: 50;
    background: rgba(255, 255, 255, 0.85);
    backdrop-filter: blur(16px);
    border-bottom: 1px solid var(--border);
  }
  .nav-in {
    max-width: 1200px;
    margin: 0 auto;
    padding: 0 40px;
    height: 64px;
    display: flex;
    align-items: center;
    gap: 36px;
  }
  .nav-home {
    display: block;
    flex-shrink: 0;
  }
  .nav-logo {
    height: 22px;
    display: block;
  }
  .nav-links {
    display: flex;
    gap: 30px;
    margin-left: 8px;
  }
  .nav-links a {
    color: var(--fg-secondary);
    font-size: 14px;
    font-weight: 500;
  }
  .nav-links a:hover {
    color: var(--fg-primary);
  }
  .nav-cta {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .nav-cta .ghost {
    color: var(--fg-secondary);
    font-size: 14px;
    font-weight: 500;
  }
  .nav-cta .ghost:hover {
    color: var(--fg-primary);
  }

  @media (max-width: 940px) {
    .nav-links {
      display: none;
    }
  }
  @media (max-width: 560px) {
    .nav-in {
      padding: 0 22px;
    }
  }
</style>
