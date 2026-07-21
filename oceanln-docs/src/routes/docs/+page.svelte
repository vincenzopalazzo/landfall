<script lang="ts">
  import { base } from '$app/paths';
  import { docsNav } from '$lib/docs-nav';

  // Cards for every page except this index.
  const cards = docsNav.flatMap((s) => s.links).filter((l) => l.slug !== '');
</script>

<svelte:head>
  <title>Documentation — OCEAN Lightning</title>
  <meta
    name="description"
    content="OCEAN Lightning documentation: the CLI, the loopback HTTP API, the web wizard, and the desktop app for receiving mining payouts over Lightning."
  />
</svelte:head>

<p class="eyebrow">Documentation</p>
<h1>OCEAN Lightning</h1>
<p class="lead">
  Payout tooling that receives your OCEAN mining rewards over the Lightning Network — self-custodial, to a
  node you run. Generate a seed, derive your mining address, create a payable BOLT12 offer, and BIP-322
  sign the OCEAN message, end to end.
</p>

<div class="callout">
  <span class="material-symbols-sharp">bolt</span>
  <p>
    New here? Start with <a href="{base}/docs/getting-started">Getting started</a> to install the tooling
    and run your first payout, then dive into the <a href="{base}/docs/cli">CLI reference</a>.
  </p>
</div>

<h2>What it is</h2>
<p>
  <strong>oceanln</strong> is a Cargo workspace with three crates over one shared core. The core holds the
  actual flow — resolve offer → load seed → derive → BIP-322 sign → provision — and every frontend is a
  thin adapter over it.
</p>

<table>
  <thead>
    <tr><th>Crate</th><th>What it is</th></tr>
  </thead>
  <tbody>
    <tr>
      <td><code>oceanln-common</code></td>
      <td>Shared core: BIP-322 signing, address derivation, seed sources, Lexe wallet/sidecar client.</td>
    </tr>
    <tr>
      <td><code>oceanln-cli</code></td>
      <td>The <code>oceanln</code> command-line tool — for humans, scripts, and AI.</td>
    </tr>
    <tr>
      <td><code>oceanln-httpd</code></td>
      <td>A local loopback HTTP server exposing the same flow to a web or desktop frontend.</td>
    </tr>
  </tbody>
</table>

<p>
  Two more frontends sit alongside the workspace: <code>oceanln-web</code> (the Svelte wizard) and
  <code>src-tauri</code> (the Tauri desktop shell). Both share one orchestration core in
  <code>oceanln-httpd::service</code>, so the HTTP handlers and the desktop IPC commands stay thin.
</p>

<h2>Explore the docs</h2>
<div class="cards">
  {#each cards as card}
    <a class="card" href="{base}/docs/{card.slug}">
      <span class="card-title">{card.title}</span>
      <span class="card-sum">{card.summary}</span>
      <span class="card-go">Read <span class="arrow">&#8594;</span></span>
    </a>
  {/each}
</div>

<style>
  .cards {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 18px;
    margin-top: 8px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 24px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    text-decoration: none;
    transition:
      border-color var(--dur-base),
      transform var(--dur-base) var(--ease-out);
  }
  .card:hover {
    border-color: var(--ocean-blue-light);
    transform: translateY(-2px);
  }
  .card-title {
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: 700;
    letter-spacing: -0.3px;
    color: var(--fg-primary);
  }
  .card-sum {
    font-size: 14.5px;
    line-height: 1.55;
    color: var(--fg-secondary);
  }
  .card-go {
    margin-top: 4px;
    font-size: 14px;
    font-weight: 600;
    color: var(--ocean-blue);
  }

  @media (max-width: 560px) {
    .cards {
      grid-template-columns: 1fr;
    }
  }
</style>
