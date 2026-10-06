<script lang="ts">
  import { base } from '$app/paths';

  const generate = `landfall generate`;

  const payout = `landfall payout \\
  --message "Configure OCEAN payout to lno1... at block 840000" \\
  --description "my pool payout" \\
  --min-amount 1000`;

  const offerDirect = `landfall payout --offer lno1... --message "<exact OCEAN message>"`;

  const offer = `landfall offer --description "OCEAN payout"   # create a payable BOLT12 offer, print it`;
</script>

<svelte:head>
  <title>CLI reference — OCEAN Lightning</title>
  <meta name="description" content="The landfall command-line tool: generate, init, offer, payout, and how the seed is resolved." />
</svelte:head>

<p class="eyebrow">Guides</p>
<h1>CLI reference</h1>
<p class="lead">
  <code>landfall</code> is the command-line frontend over <code>landfall-common</code> — for humans,
  scripts, and AI. Every command that touches your key resolves the seed the same way.
</p>

<h2><code>generate</code></h2>
<p>Generates a fresh 24-word BIP39 mnemonic (256 bits of entropy from the OS CSPRNG) and prints it <strong>once</strong>:</p>
<pre><code>{generate}</code></pre>
<p>This single seed does double duty:</p>
<ul>
  <li>feed it to <code>landfall payout</code> to derive your mining address and sign, and</li>
  <li>feed it to the Lexe sidecar as its root seed (<code>LEXE_ROOT_SEED_PATH=&lt;file&gt; lexe-sidecar</code>), so the same wallet runs your Lightning node.</li>
</ul>
<p>
  The mnemonic goes to <strong>stdout</strong>; the warning and usage hint go to stderr, so
  <code>landfall generate --json</code> yields a clean <code>&#123;"mnemonic": "..."&#125;</code>. It is
  never written to disk — write it down yourself.
</p>

<h2><code>init</code></h2>
<p>
  <code>init --generate</code> is the one-shot onboarding: generate a seed, derive the mining address,
  provision the wallet, and persist the seed to <code>~/.config/landfall/seed</code>. Drop
  <code>--generate</code> to onboard an existing seed from stdin; add <code>--dry-run</code> to derive
  without any network call. See <a href="{base}/docs/getting-started">Getting started</a> for the full
  walk-through.
</p>

<h2><code>offer</code></h2>
<p>Creates a payable BOLT12 offer on your node and prints it:</p>
<pre><code>{offer}</code></pre>
<div class="callout warn">
  <span class="material-symbols-sharp">info</span>
  <p>The offer must come from a running node — a BOLT12 offer built offline from a key is structurally valid but <strong>unpayable</strong> (no node answers invoice requests for it), so the flow uses the node's <code>create_offer</code> instead.</p>
</div>

<h2><code>payout</code></h2>
<p>One command does the whole setup. It resolves your seed, then:</p>
<ol>
  <li>derives your BIP84 mining address (<code>m/84'/0'/0'/0/0</code>) — the address you register with OCEAN, provably controlled by the same seed it signs with;</li>
  <li>asks the <strong>node</strong> to create a payable BOLT12 offer with your <code>--description</code> (via the sidecar's <code>POST /v2/node/create_offer</code>);</li>
  <li>BIP-322 signs the OCEAN <code>--message</code> <strong>verbatim</strong> with the derived key;</li>
  <li>prints the address, the offer, and the base64 signature (add <code>--json</code> for a machine-readable object).</li>
</ol>
<pre><code>{payout}</code></pre>
<p>
  Order matters: the offer is created before signing, so if the sidecar is down the flow aborts without
  using your mnemonic on a message you couldn't submit. <code>--min-amount</code> is in satoshis; omit it
  for a variable-amount offer. <code>--path</code> overrides the default derivation path.
</p>

<h3>Already have an offer? Sign for it directly</h3>
<p>
  Pass <code>--offer &lt;lno1...&gt;</code> and <code>payout</code> <strong>skips offer creation
  entirely</strong> — no sidecar is contacted, it just derives the address and BIP-322 signs the message
  (fully offline):
</p>
<pre><code>{offerDirect}</code></pre>
<p><code>--offer</code> is mutually exclusive with <code>--description</code> / <code>--min-amount</code>.</p>

<h2>Seed resolution</h2>
<p><code>init</code>, <code>offer</code>, and <code>payout</code> resolve the seed in this order, stopping at the first that yields one:</p>
<ol>
  <li><code>--seed-file &lt;path&gt;</code> — an explicit override (read, and for <code>init</code> also the write target);</li>
  <li><strong>piped stdin</strong> — <code>echo "$SEED" | landfall …</code> or the test harness;</li>
  <li>the <strong>persisted managed file</strong> — <code>$XDG_CONFIG_HOME/landfall/seed</code>, else <code>~/.config/landfall/seed</code>, if present;</li>
  <li>an interactive <strong>hidden prompt</strong> (TTY only).</li>
</ol>
<p>
  The seed file is plaintext but written <code>0600</code> (owner-only); a group/world-readable seed
  file is rejected with a <code>chmod 600</code> hint. <code>init</code> refuses to overwrite a file
  holding a <em>different</em> seed unless you pass <code>--force</code>, and <code>--no-store</code>
  skips persistence entirely for a one-off provisioning.
</p>

<h2>Thin build</h2>
<p>Drop the SDK for a smaller dependency tree — only <code>generate</code> + <code>payout</code> (the sidecar client):</p>
<pre><code>cargo build --no-default-features -p landfall-common -p landfall-cli</code></pre>
