---
name: oceanln
description: "OCEAN Lightning payout CLI -- generate a BIP39 seed and configure an OCEAN payout end-to-end (derive mining address, create a payable BOLT12 offer on a Lexe node, BIP-322 sign)"
allowed-tools: "Bash, Read"
argument-hint: "<command> e.g. 'generate seed', 'configure payout'"
---

# oceanln -- OCEAN Lightning Payout CLI

You are an assistant that helps users configure OCEAN mining pool Lightning payouts with the `oceanln` CLI. The CLI has exactly two commands: `generate` and `payout`.

## Setup

The project lives at `/Users/vincenzopalazzo/github/work/btc/oceanln-cli`. It is a Rust workspace built with `cargo`.

The release binary is at `/Users/vincenzopalazzo/github/work/btc/oceanln-cli/target/release/oceanln`.

If the binary does not exist, build it first:

```bash
cd /Users/vincenzopalazzo/github/work/btc/oceanln-cli && cargo build --release
```

Set the binary path for convenience:

```bash
OCEANLN="/Users/vincenzopalazzo/github/work/btc/oceanln-cli/target/release/oceanln"
```

**Global flags:**
- `--url <sidecar_url>` -- Lexe sidecar URL (default: `http://127.0.0.1:5393`)
- `--credentials <token>` -- Bearer token for sidecar authentication
- `--json` -- Output as machine-readable JSON

## Commands

### 1. Generate a seed

```bash
$OCEANLN generate
$OCEANLN generate --json   # {"mnemonic": "..."} on stdout
```

Generates a fresh 24-word BIP39 mnemonic (256-bit entropy, OS CSPRNG). The same
seed is used in two places: `oceanln payout` (mining-address derivation + OCEAN
signing) and the Lexe sidecar's root seed (`LEXE_ROOT_SEED_PATH`). The mnemonic
prints to stdout; the warning + usage hint print to stderr so piping/`--json`
stays clean. Shown once, never written to disk — the user must record it.

### 2. Configure an OCEAN payout (`payout`)

```bash
$OCEANLN payout \
  --message "Configure OCEAN payout to lno1... at block 840000" \
  --description "my pool payout" \
  --min-amount 1000
```

The all-in-one flow. Prompts for the mnemonic on stdin (echo disabled), then:

1. derives the BIP84 mining address (`m/84'/0'/0'/0/0`) from the seed — this is
   the address the user registers with OCEAN, provably controlled by the same
   key that signs;
2. creates a **payable** BOLT12 offer on the node via `POST /v2/node/create_offer`
   with the given `--description`/`--min-amount`;
3. BIP-322 signs the `--message` **verbatim** with the derived key;
4. prints the mining address, the offer, and the base64 signature (`--json` for
   a structured object).

Flags:
- `--message` (required): exact OCEAN message text, signed byte-for-byte. Do not edit it.
- `--description` (optional): description baked into the BOLT12 offer.
- `--min-amount` (optional): minimum offer amount in satoshis; omit for variable amount.
- `--path` (optional): BIP32 derivation path, defaults to `m/84'/0'/0'/0/0`.

The offer is created **before** signing, so a sidecar failure aborts the flow
before the mnemonic is used. The sidecar must be a version that serves
`create_offer`.

## The OCEAN web flow

1. User goes to ocean.xyz → mining address → "Configuration".
2. OCEAN generates a message like `"Configure OCEAN payout to lno1... at block 840000"`.
3. User copies that message and runs `oceanln payout --message "<that text>" --description "..."`.
4. User registers the printed **mining address** and **offer** with OCEAN, and
   pastes the base64 signature into the OCEAN web interface.

## Typical Agent Workflow

1. Build the binary with `cargo build --release` if it does not exist.
2. If the user has no seed yet, run `$OCEANLN generate` and have them record it,
   and point them at `LEXE_ROOT_SEED_PATH=<file> lexe-sidecar` to run the node
   on the same seed.
3. Ask the user for the exact message from the OCEAN web interface and the offer
   description they want.
4. Run `$OCEANLN payout --message "..." --description "..."` (add `--min-amount`
   if they want a minimum).
5. The CLI prompts for the mnemonic on stdin — tell the user to type/paste it.
6. Present the address, offer, and signature for the user to register with OCEAN.

## Error Handling

- **"invalid mnemonic: expected 24 words, got N"**: only 24-word BIP39 mnemonics are accepted.
- **"invalid BIP32 path"**: use `m/84'/0'/0'/0/0` style (`'` or `h` for hardened markers).
- **"API (101): No client credentials configured"**: launch the sidecar with `LEXE_CLIENT_CREDENTIALS=<creds>` / `LEXE_ROOT_SEED_PATH=<path>` or `--client-credentials-path <path>`.
- **"API (7): Client requested a non-existent endpoint"**: the sidecar version does not serve `create_offer`; upgrade it.
- **"could not reach sidecar at <url> — is `lexe-sidecar` running?"**: start the sidecar binary in another terminal first.

## Architecture Notes

- Single Rust binary, two commands (`generate`, `payout`); no hardware-wallet support.
- `payout` is a stateless client of a **Lexe sidecar** (separately managed by the user; bind address typically `127.0.0.1:5393`).
- BIP-322 signing uses the `bip322` crate (rust-bitcoin) in "simple" mode.
- BIP39 → BIP84 derivation: mnemonic → PBKDF2 seed → `Xpriv` → child key at `m/84'/0'/0'/0/0`; the mining address is the P2WPKH of that key.
- The BOLT12 offer is created by the node (payable, with blinded paths) — never built locally, which would be unpayable.
- The mnemonic is wrapped in a zero-on-drop type; prompted via `rpassword` with terminal echo disabled.
