<script lang="ts">
  import { base } from '$app/paths';

  const tauri = `cargo install tauri-cli --version "^2.0" --locked   # one-time
cargo tauri dev                                     # from the repo root (finds src-tauri/)`;

  const sidecar = `LEXE_ROOT_SEED_PATH=<path-to-your-mnemonic> lexe-sidecar
# or LEXE_CLIENT_CREDENTIALS=<client-credentials-from-the-Lexe-app>`;
</script>

<svelte:head>
  <title>Desktop app — OCEAN Lightning</title>
  <meta name="description" content="The Tauri desktop shell around the OCEAN Lightning wizard, Linux packaging, and the Lexe sidecar." />
</svelte:head>

<p class="eyebrow">Frontends</p>
<h1>Desktop app</h1>
<p class="lead">
  <code>src-tauri</code> wraps the same <a href="{base}/docs/wizard">oceanln-web</a> wizard in a
  <a href="https://tauri.app" target="_blank" rel="noreferrer noopener">Tauri</a> v2 window.
</p>

<h2>Native IPC, no HTTP</h2>
<p>
  There is <strong>no</strong> HTTP server, loopback port, or bearer token in the desktop build: the
  webview reaches the Rust backend over native IPC (<code>#[tauri::command]</code> ↔ <code>invoke</code>),
  and the seed lives in the OS app-data dir (e.g.
  <code>~/Library/Application Support/xyz.oceanln.desktop/seed</code>, <code>0600</code>). The commands
  are thin adapters over <code>oceanln_httpd::service</code>, so signing/seed logic is identical to the
  HTTP path.
</p>
<pre><code>{tauri}</code></pre>
<p>
  <code>cargo tauri dev</code> builds <code>oceanln-web</code>, opens the window, and hot-reloads. It's
  its own Cargo workspace (heavy native deps), excluded from the root so the core Rust CI is unaffected.
</p>

<h2>Linux packages (.deb / .rpm / .AppImage)</h2>
<p>
  Tauri's Linux bundlers link <code>webkit2gtk</code>/GTK and shell out to <code>dpkg-deb</code>,
  <code>rpmbuild</code>, and <code>appimagetool</code>, so the Linux packages <strong>must be built on
  Linux</strong> — they cannot be cross-built from macOS. Two ways:
</p>
<ul>
  <li><strong>CI</strong> — <code>.github/workflows/desktop-linux.yml</code> builds the <code>.deb</code>, <code>.rpm</code>, and <code>.AppImage</code> on an Ubuntu runner and uploads them as artifacts (attaching them to a Release on a <code>v*</code> tag). Trigger it manually or by pushing a version tag.</li>
  <li><strong>Locally, with Docker</strong> — <code>scripts/build-linux-desktop.sh</code> builds the same three formats inside an <code>ubuntu:22.04</code> container and drops them in <code>dist-linux/</code>. It reuses the static <code>oceanln-web/dist</code> and a container-internal Rust target, so it won't clobber your host build.</li>
</ul>
<p>macOS <code>.app</code> / <code>.dmg</code> come from <code>cargo tauri build</code> on macOS. Signed/notarized installers are not yet wired up.</p>

<h2>The Lexe sidecar</h2>
<p>
  The CLI <code>payout</code> command can talk to a
  <a href="https://github.com/lexe-app/lexe-public" target="_blank" rel="noreferrer noopener">Lexe sidecar</a>
  running locally on <code>127.0.0.1:5393</code> (or pass <code>--url</code>). Launch it with the same
  seed <code>generate</code> produced:
</p>
<pre><code>{sidecar}</code></pre>
<div class="callout warn">
  <span class="material-symbols-sharp">warning</span>
  <p>The sidecar must be a version that serves <code>POST /v2/node/create_offer</code>. If it isn't running you'll see: <code>could not reach sidecar at http://127.0.0.1:5393</code>.</p>
</div>
<p>
  Note the <code>oceanln-httpd</code> server uses <code>--sidecar-url</code> / <code>--sidecar-credentials</code>
  instead of the CLI's <code>--url</code> / <code>--credentials</code> — see the
  <a href="{base}/docs/http-api">HTTP API</a>.
</p>
