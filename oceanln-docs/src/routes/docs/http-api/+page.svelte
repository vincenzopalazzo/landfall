<script lang="ts">
  const run = `oceanln-httpd --seed-file ./seed.txt --allow-origin http://localhost:5173
# oceanln-httpd listening on http://127.0.0.1:7762
# bearer token: <64 hex chars>      # printed once unless you pass --token`;

  const sidecar = `oceanln-httpd --seed-file ./seed.txt \\
  --sidecar-url http://127.0.0.1:5393 --sidecar-credentials <token>`;

  const curl = `curl -s http://127.0.0.1:7762/payout \\
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \\
  -d '{"message":"<exact OCEAN message embedding the offer>","offer":"lno1..."}'`;

  const endpoints = [
    { m: 'GET /health', body: '—', ret: '{"status":"ok"}' },
    { m: 'GET /status', body: '—', ret: '{configured, mining_address?, offer?}' },
    { m: 'POST /generate', body: '—', ret: '{mnemonic, mining_address}' },
    { m: 'POST /import', body: '{mnemonic, force?}', ret: '{mining_address}' },
    { m: 'POST /seed/reveal', body: '—', ret: '{mnemonic}' },
    { m: 'POST /payout', body: '{message, offer?, description?, min_amount?, path?}', ret: '{address, offer, message, signature}' },
    { m: 'POST /offer', body: '{description?, min_amount?}', ret: '{offer}' },
    { m: 'POST /init', body: '{path?}', ret: '{mining_address, provisioned}' },
    { m: 'GET /payouts?limit=', body: '—', ret: '[OceanPayout…] newest first' },
    { m: 'GET /node', body: '—', ret: 'node status + balances' },
    { m: 'GET /activity?limit=', body: '—', ret: '[Activity…] every payment' },
    { m: 'POST /invoice', body: '{amount_sats?, description?}', ret: 'BOLT11 invoice' },
    { m: 'POST /pay', body: '{payable, amount_sats?, note?}', ret: 'payment summary' },
    { m: 'GET /ocean/statsnap/:address', body: '—', ret: 'OCEAN public API proxy' },
    { m: 'GET /ocean/earnpay/:address', body: '—', ret: 'OCEAN public API proxy' },
    { m: 'GET /ocean/user_hashrate/:address', body: '—', ret: 'OCEAN public API proxy' },
    { m: 'GET /ocean/pool_stat', body: '—', ret: 'OCEAN public API proxy' }
  ];
</script>

<svelte:head>
  <title>HTTP API — OCEAN Lightning</title>
  <meta name="description" content="The oceanln-httpd loopback HTTP server: endpoints, bearer auth, Origin allowlist, and its security model." />
</svelte:head>

<p class="eyebrow">Guides</p>
<h1>HTTP API</h1>
<p class="lead">
  For a web app or desktop frontend, run <code>oceanln-httpd</code> instead of shelling out to the CLI.
  It's a thin loopback HTTP transport over the same <code>oceanln-common</code> core, so a UI can drive
  <code>payout</code> / <code>offer</code> / <code>init</code> over <code>127.0.0.1</code>.
</p>

<h2>Run the server</h2>
<pre><code>{run}</code></pre>
<p>
  The <strong>seed stays server-side</strong>: the server reads the 24 words from
  <code>--seed-file</code> per request and signs in-process. Only the onboarding and backup routes
  (<code>/generate</code>, <code>/import</code>, <code>/seed/reveal</code>) ever return the phrase;
  signing and every wallet operation keep it on the server.
</p>

<h2>Security model</h2>
<p>Because a browser is a supported client, the loopback port is guarded:</p>
<ul>
  <li>binds a <strong>loopback address only</strong> (refuses a routable <code>--bind</code>);</li>
  <li>requires <code>Authorization: Bearer &lt;token&gt;</code> on every endpoint except <code>/health</code> (token auto-generated and printed, or set with <code>--token</code>);</li>
  <li>enforces an <strong>Origin allowlist</strong> (<code>--allow-origin</code>, repeatable) — blocks cross-origin browser calls and DNS-rebinding;</li>
  <li>validates the <code>Host</code> header is loopback;</li>
  <li>the Lexe sidecar URL/credentials are <strong>server-side config</strong> (<code>--sidecar-url</code> / <code>--sidecar-credentials</code>), never taken from a request body (no SSRF).</li>
</ul>
<pre><code>{sidecar}</code></pre>

<h2>Endpoints</h2>
<p>
  All JSON; all but <code>/health</code> need the bearer token. In <code>--no-auth</code> mode only the
  read-only routes (<code>/status</code>, <code>/payouts</code>, <code>/node</code>, <code>/activity</code>,
  <code>/ocean/*</code>) answer without one; every seed-, key- or state-touching route still returns
  <code>403</code> until the server is restarted with a token.
</p>
<table>
  <thead>
    <tr><th>Method + path</th><th>Body</th><th>Returns</th></tr>
  </thead>
  <tbody>
    {#each endpoints as e}
      <tr>
        <td><code>{e.m}</code></td>
        <td>{e.body === '—' ? '—' : ''}{#if e.body !== '—'}<code>{e.body}</code>{/if}</td>
        <td><code>{e.ret}</code></td>
      </tr>
    {/each}
  </tbody>
</table>

<div class="callout warn">
  <span class="material-symbols-sharp">shield</span>
  <p>
    <code>/generate</code> and <code>/import</code> are the deliberate exceptions to "the seed never
    crosses the wire": <code>/generate</code> creates a fresh phrase, persists it, and reveals it
    <strong>exactly once</strong> so the user can back it up; <code>/import</code> accepts an existing
    phrase. Both refuse with <code>409</code> if a seed file already exists. Replacing a wallet must go
    through <code>/import</code> with <code>&#123;"force":true&#125;</code>.
  </p>
</div>

<h2>Example: sign for an existing offer</h2>
<p>
  <code>/payout</code> mirrors the CLI: pass <code>offer</code> to sign for an existing offer fully
  offline (it must be embedded in <code>message</code>), or omit it to have the configured sidecar create
  one first.
</p>
<pre><code>{curl}</code></pre>
