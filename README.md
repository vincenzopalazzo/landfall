# oceanln

OCEAN Lightning payout tooling: generate a BIP39 seed, and configure an OCEAN
payout end-to-end (derive the mining address, create a payable BOLT12 offer on a
[Lexe](https://lexe.app) node, and BIP-322 sign the OCEAN message).

## Workspace layout

A Cargo workspace with three crates over one shared core:

| crate | what it is |
|---|---|
| `oceanln-common` | shared core: BIP-322 signing, address derivation, seed sources, Lexe wallet/sidecar client |
| `oceanln-cli` | the `oceanln` command-line tool (for humans / scripts / AI) |
| `oceanln-httpd` | a local loopback HTTP server exposing the same flow to a web app or desktop frontend |

`oceanln-cli` and `oceanln-httpd` are independent frontends over
`oceanln-common`; neither depends on the other. The CLI keeps its offline,
in-process path; the server is what a UI talks to.

## Build

```sh
cargo build --release --workspace
```

Binaries: `target/release/oceanln` (CLI) and `target/release/oceanln-httpd`
(server).

## Use

### Generate a seed

```sh
oceanln generate
```

Generates a fresh 24-word BIP39 mnemonic (256 bits of entropy from the OS
CSPRNG) and prints it **once**. This single seed does double duty:

- feed it to `oceanln payout` to derive your mining address and sign, and
- feed it to the Lexe sidecar as its root seed
  (`LEXE_ROOT_SEED_PATH=<file with these words> lexe-sidecar`), so the same
  wallet runs your Lightning node.

The mnemonic goes to **stdout**; the warning and usage hint go to stderr, so
`oceanln generate --json` / piping yields a clean `{"mnemonic": "..."}` (or the
bare words). It is never written to disk — write it down yourself.

### Configure an OCEAN payout (end-to-end)

```sh
oceanln payout \
  --message "Configure OCEAN payout to lno1... at block 840000" \
  --description "my pool payout" \
  --min-amount 1000
```

One command does the whole setup. It resolves your seed (see
[Seed resolution](#seed-resolution) — a persisted seed file, a pipe, or an
interactive hidden prompt), then:

1. derives your BIP84 mining address (`m/84'/0'/0'/0/0`) — the address you
   register with OCEAN, provably controlled by the same seed it signs with;
2. asks the **node** to create a payable BOLT12 offer with your `--description`
   (via the sidecar's `POST /v2/node/create_offer`), so the offer has real
   blinded paths back to your node and can actually receive rewards;
3. BIP-322 signs the OCEAN `--message` **verbatim** with the derived key;
4. prints the address, the offer, and the base64 signature (add `--json` for a
   machine-readable object).

Order matters: the offer is created before signing, so if the sidecar is down
the flow aborts without using your mnemonic on a message you couldn't submit.

> The offer must come from a running node — a BOLT12 offer built offline from a
> key is structurally valid but **unpayable** (no node answers invoice requests
> for it), so `payout` deliberately uses the node's `create_offer` instead.

`--min-amount` is in satoshis; omit it for a variable-amount offer. `--path`
overrides the default derivation path.

#### Already have an offer? Sign for it directly (`--offer`)

OCEAN's flow is offer-first: you give it an offer, it generates the message
embedding that offer, then you sign. Pass `--offer <lno1...>` and `payout`
**skips offer creation entirely** — no sidecar is contacted, it just derives
the address and BIP-322 signs the message (fully offline):

```sh
oceanln payout --offer lno1... --message "<exact OCEAN message>"
```

So the real OCEAN sequence is: create/register the offer (your node, the Lexe
app, or a `payout` run without `--offer`), paste it into OCEAN to get the
message, then sign that message here with `--offer`. `--offer` is mutually
exclusive with `--description`/`--min-amount`.

### In-process Lexe wallet (no sidecar) — default

By default oceanln embeds the published [`lexe`](https://crates.io/crates/lexe)
SDK and runs the wallet in-process — no separate `lexe-sidecar` needed. Install
the CLI from the `oceanln-cli` workspace crate (the repo root is a virtual
workspace, so `--path .` won't work); this gives you the full CLI, with `init`
and `offer`:

```sh
cargo install --path oceanln-cli            # CLI binary `oceanln`
# optional: the local HTTP server for a web/desktop frontend
cargo install --path oceanln-httpd          # binary `oceanln-httpd`

oceanln init --generate   # one shot: generate seed + provision wallet + print mining address
oceanln offer --description "OCEAN payout"   # create a payable BOLT12 offer, print it
```

`init --generate` does the whole onboarding at once — generates a fresh 24-word
seed (printed once), derives the **mining address** to register with OCEAN, and
provisions the onchain Lexe wallet. Drop `--generate` to onboard an existing
seed read from stdin. Add `--dry-run` to derive the seed + mining address
**without** provisioning (no network) — handy for testing. `--path` overrides
the address derivation path.

`init` also **persists the seed** to `~/.config/oceanln/seed` (0600) so the
subsequent `offer` / `payout` runs don't re-prompt — see
[Seed resolution](#seed-resolution). The seed is saved *before* the network
call, so a provisioning failure never loses a freshly generated seed.

Full OCEAN flow, sidecar-free — `init` persists the seed, so the later steps
read it automatically (no piping):

```sh
# Create the wallet (seed persisted to ~/.config/oceanln/seed) + print address.
oceanln init --generate --json > wallet.json
# {"mnemonic": "...", "mining_address": "bc1q...", "provisioned": true, "seed_file": "/home/you/.config/oceanln/seed"}
python3 -c 'import sys,json;print("mining address:", json.load(sys.stdin)["mining_address"])' < wallet.json

# Create the offer, then sign the OCEAN message for it — no seed piping needed.
OFFER=$(oceanln offer --json --description "OCEAN payout" \
          | python3 -c 'import sys,json;print(json.load(sys.stdin)["offer"])')
# register the mining address + $OFFER on ocean.xyz -> copy the message it gives you
oceanln payout --offer "$OFFER" --message "<exact OCEAN message>"
```

`init` is headless (no app, no Google Drive) — it registers with Lexe's backend
and provisions, exactly like `lexe init` (verified end-to-end on mainnet).

#### Seed resolution

`init`, `offer`, and `payout` resolve the seed in this order, stopping at the
first that yields one:

1. `--seed-file <path>` — an explicit override (read, and for `init` also the
   write target);
2. **piped stdin** — `echo "$SEED" | oceanln …` or the test harness;
3. the **persisted managed file** — `$XDG_CONFIG_HOME/oceanln/seed`, else
   `~/.config/oceanln/seed`, if present;
4. an interactive **hidden prompt** (TTY only).

The seed file is plaintext but written `0600` (owner-only); a group/world-
readable seed file is rejected with a `chmod 600` hint. `init` refuses to
overwrite a file holding a *different* seed unless you pass `--force` (an
identical write is a no-op), and `--no-store` skips persistence entirely for a
one-off provisioning.

**Thin build:** `cargo build --no-default-features -p oceanln-common -p
oceanln-cli` drops the SDK for a smaller dependency tree — only `generate` +
`payout` (the sidecar client). See issue #3 for the migration notes.

## Local HTTP server (`oceanln-httpd`)

For a web app or desktop frontend, run `oceanln-httpd` instead of shelling out
to the CLI. It's a thin loopback HTTP transport over the same `oceanln-common`
core, so a UI can drive `payout` / `offer` / `init` over `127.0.0.1`.

```sh
oceanln-httpd --seed-file ./seed.txt --allow-origin http://localhost:5173
# oceanln-httpd listening on http://127.0.0.1:7762
# bearer token: <64 hex chars>      # printed once unless you pass --token
```

The **seed never crosses the HTTP boundary**: the server reads the 24 words from
`--seed-file` per request, signs in-process, and never echoes them back. Seed
*generation* stays a human-witnessed CLI operation (`oceanln generate` /
`oceanln init --generate`) and is deliberately not exposed over HTTP.

Because a browser is a supported client, the loopback port is guarded:

- binds a **loopback address only** (refuses a routable `--bind`);
- requires `Authorization: Bearer <token>` on every endpoint except `/health`
  (token auto-generated and printed, or set with `--token`);
- enforces an **`Origin` allowlist** (`--allow-origin`, repeatable) — blocks
  cross-origin browser calls and DNS-rebinding;
- validates the `Host` header is loopback;
- the Lexe sidecar URL/credentials are **server-side config** (`--sidecar-url` /
  `--sidecar-credentials`), never taken from a request body (no SSRF).

Point the server at a non-default sidecar with the server's own flags (these
differ from the CLI's `--url` / `--credentials`):

```sh
oceanln-httpd --seed-file ./seed.txt \
  --sidecar-url http://127.0.0.1:5393 --sidecar-credentials <token>
```

Endpoints (all JSON; all but `/health` need the bearer token):

| method + path | body | returns |
|---|---|---|
| `GET /health` | — | `{"status":"ok"}` |
| `POST /generate` | — | `{mnemonic, mining_address}` |
| `POST /import` | `{mnemonic, force?}` | `{mining_address}` |
| `POST /payout` | `{message, offer?, description?, min_amount?, path?}` | `{address, offer, message, signature}` |
| `POST /offer` | `{description?, min_amount?}` | `{offer}` |
| `POST /init` | `{path?}` | `{mining_address, provisioned}` |

`/generate` and `/import` exist for the onboarding wizard and are the deliberate
exceptions to "the seed never crosses the wire": `/generate` creates a fresh
24-word phrase, persists it to the seed file, and **reveals it exactly once** in
the response so the user can back it up; `/import` accepts an existing phrase.
Both **refuse with `409` if a seed file already exists**. `/generate` has no
`force` — generating over an existing wallet would irreversibly destroy it, so
replacing a wallet must go through `/import` (`{"force":true}`), a deliberate
user-supplied action. They stay gated by the loopback bind + bearer token +
Origin allowlist; everything else (signing, wallet ops) keeps the seed
server-side.

`/payout` mirrors the CLI: pass `offer` to sign for an existing offer fully
offline (it must be embedded in `message`), or omit it to have the configured
sidecar create one first.

```sh
curl -s http://127.0.0.1:7762/payout \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"message":"<exact OCEAN message embedding the offer>","offer":"lno1..."}'
```

## Web wizard (`oceanln-web`)

`oceanln-web/` is the Svelte onboarding wizard that drives `oceanln-httpd` from a
browser: create/import a recovery phrase → back up → confirm → create wallet
(description → BOLT12 offer) → BIP-322 sign → copy the three artifacts. It also
has a profile (1→n payout addresses linked to offers, reveal phrase) and an
illustrative payout dashboard + MCP panel (those are mocks — no backend yet).
It's a static SPA, structured so a later Tauri shell can bundle it unchanged.

Run both with the dev script:

```sh
scripts/dev.sh          # builds + runs oceanln-httpd, then `npm run dev` in oceanln-web
# open http://localhost:5173
```

Or manually:

```sh
oceanln-httpd --seed-file ./seed --token <tok> --allow-origin http://localhost:5173 &
cd oceanln-web && npm install
VITE_OCEANLN_BASE=http://127.0.0.1:7762 VITE_OCEANLN_TOKEN=<tok> npm run dev
```

The wizard reaches the server cross-origin, so the server's `--allow-origin`
must include the Vite origin (`http://localhost:5173`); the bearer token is
injected via `VITE_OCEANLN_TOKEN` (or pasted into the in-app settings panel).
The phrase-generation and signing steps work offline; the wallet/offer steps
need `oceanln-httpd` to reach a Lexe node. `npm run build` emits static assets.

### The Lexe sidecar

The CLI `payout` command talks to a
[Lexe sidecar](https://github.com/lexe-app/lexe-public) running locally on
`127.0.0.1:5393` (or pass `--url`). The flags below are the **CLI** flags; the
`oceanln-httpd` server uses `--sidecar-url` / `--sidecar-credentials` instead
(see above). Launch the sidecar with the same seed `generate` produced:

```sh
LEXE_ROOT_SEED_PATH=<path-to-your-mnemonic> lexe-sidecar
# or LEXE_CLIENT_CREDENTIALS=<client-credentials-from-the-Lexe-app>
```

Add `--credentials <token>` to send a `Bearer` header to the sidecar. If the
sidecar isn't running, you'll see:

```
error: could not reach sidecar at http://127.0.0.1:5393 — is `lexe-sidecar` running?
```

The sidecar must be a version that serves `POST /v2/node/create_offer`.

## License

MIT.
