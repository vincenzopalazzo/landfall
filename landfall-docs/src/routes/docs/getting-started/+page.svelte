<script lang="ts">
  import { base } from '$app/paths';

  const build = `cargo build --release --workspace
# Binaries: target/release/landfall (CLI) and target/release/landfall-httpd (server)`;

  const install = `cargo install --path landfall-cli      # CLI binary \`landfall\`
# optional: the local HTTP server for a web/desktop frontend
cargo install --path landfall-httpd    # binary \`landfall-httpd\``;

  const initGen = `landfall init --generate
# one shot: generate seed + provision wallet + print mining address`;

  const fullFlow = `# Create the wallet (seed persisted to ~/.config/landfall/seed) + print address.
landfall init --generate --json > wallet.json
# {"mnemonic":"...","mining_address":"bc1q...","provisioned":true,"seed_file":"/home/you/.config/landfall/seed"}

# Create the offer, then sign the OCEAN message for it — no seed piping needed.
OFFER=$(landfall offer --json --description "OCEAN payout" \\
          | python3 -c 'import sys,json;print(json.load(sys.stdin)["offer"])')

# Register the mining address + $OFFER on ocean.xyz, copy the message it gives you:
landfall payout --offer "$OFFER" --message "<exact OCEAN message>"`;
</script>

<svelte:head>
  <title>Getting started — OCEAN Lightning</title>
  <meta name="description" content="Install the OCEAN Lightning tooling, build the workspace, and run your first end-to-end payout." />
</svelte:head>

<p class="eyebrow">Overview</p>
<h1>Getting started</h1>
<p class="lead">
  Install the tooling, build the workspace, and run the full onboarding — generate a seed, derive your
  mining address, create a payable BOLT12 offer, and BIP-322 sign the OCEAN message.
</p>

<h2>Build from source</h2>
<p>The repo root is a Cargo workspace. Build every crate at once:</p>
<pre><code>{build}</code></pre>

<h2>Install the CLI</h2>
<p>
  By default landfall embeds the published <a href="https://crates.io/crates/lexe" target="_blank" rel="noreferrer noopener">lexe</a>
  SDK and runs the wallet in-process — no separate <code>lexe-sidecar</code> needed. Install from the
  <code>landfall-cli</code> crate (the repo root is a virtual workspace, so <code>--path .</code> won't
  work):
</p>
<pre><code>{install}</code></pre>

<div class="callout">
  <span class="material-symbols-sharp">terminal</span>
  <p>The in-process build gives you the full CLI, including <code>init</code> and <code>offer</code>. See the <a href="{base}/docs/cli">CLI reference</a> for every command.</p>
</div>

<h2>One-shot onboarding</h2>
<p>
  <code>init --generate</code> does the whole onboarding at once — it generates a fresh 24-word seed
  (printed once), derives the <strong>mining address</strong> to register with OCEAN, and provisions the
  on-chain Lexe wallet.
</p>
<pre><code>{initGen}</code></pre>
<p>
  It also <strong>persists the seed</strong> to <code>~/.config/landfall/seed</code> (<code>0600</code>),
  so later <code>offer</code> / <code>payout</code> runs don't re-prompt. The seed is saved
  <em>before</em> the network call, so a provisioning failure never loses a freshly generated seed. Add
  <code>--dry-run</code> to derive the seed and mining address without provisioning (no network).
</p>

<div class="callout warn">
  <span class="material-symbols-sharp">key</span>
  <p>The mnemonic is printed <strong>once</strong> and never written anywhere but the seed file you control. Write it down — it is the root of both your mining key and your Lightning node.</p>
</div>

<h2>Full flow, sidecar-free</h2>
<p>
  Because <code>init</code> persists the seed, the later steps read it automatically — no piping between
  commands:
</p>
<pre><code>{fullFlow}</code></pre>
<p>
  OCEAN's flow is offer-first: you give it an offer, it generates the message embedding that offer, then
  you sign. Passing <code>--offer</code> makes <code>payout</code> skip offer creation entirely and sign
  fully offline. Next, read the full <a href="{base}/docs/cli">CLI reference</a> or wire a UI to the
  <a href="{base}/docs/http-api">HTTP API</a>.
</p>
