<script lang="ts">
  import { base } from '$app/paths';

  const devScript = `scripts/dev.sh          # builds + runs landfall-httpd, then \`npm run dev\` in landfall-web
# open http://localhost:5173`;

  const manual = `landfall-httpd --seed-file ./seed --token <tok> --allow-origin http://localhost:5173 &
cd landfall-web && npm install
VITE_LANDFALL_BASE=http://127.0.0.1:7762 VITE_LANDFALL_TOKEN=<tok> npm run dev`;
</script>

<svelte:head>
  <title>Web wizard — OCEAN Lightning</title>
  <meta name="description" content="The landfall-web Svelte onboarding wizard, live payout dashboard, and the local MCP panel." />
</svelte:head>

<p class="eyebrow">Frontends</p>
<h1>Web wizard</h1>
<p class="lead">
  <code>landfall-web</code> is the Svelte onboarding wizard that drives <a href="{base}/docs/http-api">landfall-httpd</a>
  from a browser: create or import a recovery phrase → back up → confirm → create wallet → BIP-322 sign →
  copy the three artifacts.
</p>

<h2>What it includes</h2>
<ul>
  <li>The <strong>onboarding wizard</strong> — recovery phrase, backup, wallet creation (the node mints a BOLT12 offer), BIP-322 signing, and copy-out of the address, offer, and signature.</li>
  <li>A <strong>profile</strong> — one-to-many payout addresses linked to offers, with re-reveal of the phrase.</li>
  <li>A <strong>live payout dashboard</strong> that reads the public OCEAN API (<code>https://api.ocean.xyz/v1</code>, browser-direct via CORS) keyed by your payout address(es): real hashrate, unpaid balance, and the on-chain payouts table.</li>
  <li>An <strong>MCP panel</strong> showing how to run the local, read-only <code>landfall-mcp</code> proxy over <code>landfall-httpd</code> — nothing hosted or exposed.</li>
</ul>
<p>
  It's a static SPA, bundled unchanged by the Tauri desktop shell. The transport is chosen at runtime —
  HTTP (<code>src/lib/api.ts</code>) in the browser, native IPC (<code>src/lib/tauri.ts</code>) in the
  desktop app.
</p>

<h2>Run it in development</h2>
<p>Run both the server and the Vite dev server with the dev script:</p>
<pre><code>{devScript}</code></pre>
<p>Or manually:</p>
<pre><code>{manual}</code></pre>

<div class="callout">
  <span class="material-symbols-sharp">lan</span>
  <p>
    The wizard reaches the server cross-origin, so the server's <code>--allow-origin</code> must include
    the Vite origin (<code>http://localhost:5173</code>). The bearer token is injected via
    <code>VITE_LANDFALL_TOKEN</code>, or pasted into the in-app settings panel.
  </p>
</div>

<p>
  The phrase-generation and signing steps work offline; the wallet/offer steps need
  <code>landfall-httpd</code> to reach a Lexe node. <code>npm run build</code> emits static assets. To ship
  it as a native window instead, see the <a href="{base}/docs/desktop">desktop app</a>.
</p>
