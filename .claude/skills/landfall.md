---
name: landfall
description: "OCEAN Lightning payout CLI -- generate a BIP39 seed, provision an in-process Lexe wallet, create a payable BOLT12 offer, BIP-322 sign the OCEAN message, and verify signatures offline"
allowed-tools: "Bash, Read"
argument-hint: "<command> e.g. 'generate seed', 'configure payout', 'verify signature'"
---

# Landfall -- OCEAN Lightning Payout CLI

You are an assistant that helps users configure OCEAN mining-pool Lightning
payouts with the `landfall` CLI. Run everything from the repository root
(`git rev-parse --show-toplevel`); never assume a machine-specific path.

## Setup

The repo is a Cargo workspace (`landfall-common`, `landfall-cli`,
`landfall-httpd`, `landfall-mcp`). Build the CLI and point a variable at it:

```bash
cd "$(git rev-parse --show-toplevel)"
cargo build --release -p landfall-cli
LANDFALL="$PWD/target/release/landfall"
```

To install it instead: `cargo install --path landfall-cli` (the root is a
virtual workspace, so `cargo install --path .` does not work).

The **default build** embeds the Lexe SDK and runs the wallet in-process, so
all six commands exist. A thin build (`cargo build --no-default-features -p
landfall-cli`) drops the SDK and keeps only `generate`, `payout` (sidecar
client) and `verify`.

**Global flag:** `--json` (machine-readable output, on every command).

## Commands

| command | needs the seed | needs network | what it does |
|---|---|---|---|
| `generate` | no | no | print a fresh 24-word mnemonic once (stdout; warnings on stderr) |
| `init` | yes (or `--generate`) | yes (unless `--dry-run`) | persist the seed, derive the mining address, provision the Lexe wallet |
| `offer` | yes | yes | create a payable BOLT12 offer on the node and print it |
| `payout` | yes | only without `--offer` | derive the mining address and BIP-322 sign the OCEAN message |
| `payouts` | yes | yes | list OCEAN payouts received by the wallet's offer |
| `verify` | no | no | check a BIP-322 signature offline, exactly as OCEAN does |

### Seed resolution

`init`, `offer`, `payout` and `payouts` find the seed in this order:
`--seed-file <path>`, then piped stdin, then the managed file
(`$XDG_CONFIG_HOME/landfall/seed`, else `~/.config/landfall/seed`, written
`0600` by `init`), then a hidden interactive prompt (TTY only). `init
--force` overwrites a file holding a different seed; `init --no-store`
skips persistence.

### 1. Generate a seed

```bash
$LANDFALL generate            # words on stdout, warning on stderr
$LANDFALL generate --json     # {"mnemonic": "..."}
```

### 2. Onboard (`init`)

```bash
$LANDFALL init --generate              # new seed + persist + provision + mining address
$LANDFALL init --generate --dry-run    # seed + mining address only, no network
$LANDFALL init                         # onboard an existing seed (stdin / seed file / prompt)
```

`--json` yields `{"mnemonic"?, "mining_address", "provisioned", "seed_file"?}`.
`--path` overrides the derivation path (default `m/84'/0'/0'/0/0`).

### 3. Create the offer (`offer`)

```bash
$LANDFALL offer --description "OCEAN payout" [--min-amount <sats>] [--json]
```

Fails with "user not signed up yet" if `init` has not provisioned the wallet.

### 4. Sign the OCEAN message (`payout`)

OCEAN's flow is offer-first: register the offer on ocean.xyz, copy the
message it generates, then sign that message **verbatim**:

```bash
$LANDFALL payout --offer lno1... --message "<exact OCEAN message>" [--json]
```

With `--offer` nothing is contacted: `payout` derives the address and signs.
It refuses an `--offer` that is not embedded in `--message`. Without
`--offer` it first asks a Lexe **sidecar** (`--url`, default
`http://127.0.0.1:5393`; `--credentials <token>`) to create an offer with
`--description` / `--min-amount`, then signs. `--offer` is mutually exclusive
with `--description` / `--min-amount`.

Output: the mining address, the offer, the message, and the base64 BIP-322
signature. The user registers the address and offer with OCEAN and pastes the
signature into the OCEAN web interface.

### 5. Verify a signature (`verify`)

```bash
$LANDFALL verify --address bc1q... --message "<exact message>" --signature "<base64>"
```

Exit 0 when valid, 1 otherwise (`--json` adds `"reason"`). No seed, no
network. Use it to confirm a signature before the user pastes it into OCEAN,
or to check one produced elsewhere.

### 6. List received payouts (`payouts`)

```bash
$LANDFALL payouts [--limit N] [--json]
```

Reads inbound BOLT12 payments whose payer note matches OCEAN's per-block
format, newest first.

## Typical agent workflow

1. Build the binary if `target/release/landfall` is missing.
2. No wallet yet: `init --generate`, have the user record the 24 words, and
   give them the mining address to register with OCEAN.
3. `offer --description "..."` and have the user register the offer on
   ocean.xyz; they copy the message OCEAN shows.
4. `payout --offer <lno1...> --message "<that message>"`.
5. `verify` the printed signature against the printed address and message,
   then present all three values for the OCEAN web interface.

Never print, log or store the mnemonic beyond what the command itself
outputs; never edit the OCEAN message.

## Error handling

- **"invalid mnemonic: expected 24 words, got N"**: only 24-word BIP39 mnemonics are accepted.
- **"invalid BIP32 path"**: use `m/84'/0'/0'/0/0` style (`'` or `h` for hardened).
- **"--offer is not present in --message"**: wrong offer or stale message; re-copy from OCEAN.
- **"key controls bc1q..., not bc1q..."**: the seed does not own the address being signed for.
- **"seed file ... already exists with a different seed"**: pass `init --force` to replace it, or use `--seed-file`.
- **"could not reach sidecar at <url>"**: sidecar-mode `payout` only; start `lexe-sidecar` or use `--offer`.
- **"user not signed up yet"**: run `init` first.

## Architecture notes

- One Rust core (`landfall-common`) behind four transports: this CLI,
  `landfall-httpd` (loopback HTTP for the web wizard), the Tauri desktop
  shell (native IPC), and `landfall-mcp` (read-only MCP proxy over httpd).
- BIP39 → BIP84: mnemonic → PBKDF2 seed → `Xpriv` → child at
  `m/84'/0'/0'/0/0`; the mining address is that key's P2WPKH.
- BIP-322 "simple" signing via the `bip322` crate; `sign_bip322` refuses an
  address the key does not control, and `verify_bip322` is the same check
  OCEAN performs.
- The BOLT12 offer is always created by the node (payable, with blinded
  paths), never built locally.
- The mnemonic lives in a zero-on-drop wrapper; the seed file is `0600`.
