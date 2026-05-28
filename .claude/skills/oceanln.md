---
name: oceanln
description: "OCEAN Lightning payout CLI -- configure payouts with BIP-322 signing from a BIP39 mnemonic, manage Lexe Lightning node (balance, invoices, payments)"
allowed-tools: "Bash, Read"
argument-hint: "<command> e.g. 'configure payout', 'sign message', 'check balance', 'create invoice 5000', 'pay lnbc...'"
---

# oceanln -- OCEAN Lightning Payout CLI

You are an assistant that helps users interact with the `oceanln` CLI for managing OCEAN mining pool Lightning payouts and a Lexe Lightning node.

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

### 1. BIP-322 Message Signing (OCEAN payout configuration)

The primary use case. The OCEAN web flow is:

1. User goes to ocean.xyz → mining address → "Configuration"
2. User pastes their BOLT12 offer and selects a block height
3. OCEAN generates a message like: `"Configure OCEAN payout to lno1... at block 840000"`
4. User copies that message and signs it with this CLI **using their BIP39 mnemonic**
5. User pastes the base64 signature back into the OCEAN web interface

**Sign a message:**

```bash
$OCEANLN sign \
  --message "Configure OCEAN payout to lno1... at block 840000" \
  --address bc1q... \
  --path "m/84'/0'/0'/0/0"
```

- `--message` (required): exact OCEAN message text
- `--address` (required): the P2WPKH (`bc1q...`) Bitcoin address registered with OCEAN
- `--path` (optional): BIP32 derivation path, defaults to `m/84'/0'/0'/0/0`

The CLI will:
- **Prompt for the 24-word BIP39 mnemonic on stdin (terminal echo disabled)**
- Derive the BIP84 child private key at the given path
- Construct the BIP-322 "simple" signature
- Output the base64-encoded witness

**Security:** the mnemonic is held in a zeroize-on-drop wrapper. It is never logged, never echoed, never written to disk. Only 24-word mnemonics are accepted (matches Lexe RootSeed expectations).

### 2. OCEAN Payout Configuration (no signing)

Display the configuration details and the message to sign externally:

```bash
$OCEANLN configure \
  --offer lno1qgsq... \
  --address bc1q... \
  --message "Configure OCEAN payout to lno1... at block 840000"
```

If `--offer` is omitted, the CLI fetches the BOLT12 offer from the connected Lexe node (`GET /v2/node/offer`).

### 3. Lexe Lightning Node Operations

**Check node info (balance, channels):**

```bash
$OCEANLN info
$OCEANLN --json info
```

**Create a BOLT11 invoice:**

```bash
$OCEANLN invoice 5000 "donation"
```

- First argument: amount in satoshis
- Second argument (optional): description

**Pay a BOLT11 invoice:**

```bash
$OCEANLN pay lnbc50n1pn...
```

**Look up a payment by index:**

```bash
$OCEANLN payment "0000001772349163844-ln_abc"
```

**Health check (verify sidecar is running):**

```bash
$OCEANLN health
```

## Typical Agent Workflow

When the user asks to "configure OCEAN Lightning payouts" or "sign my OCEAN message":

1. Check if the binary exists; build with `cargo build --release` if needed
2. Run `$OCEANLN health` to verify the Lexe sidecar is running (if wallet commands are needed)
3. Run `$OCEANLN info` to show current node status
4. Ask the user for the message from the OCEAN web interface
5. Ask the user for their Bitcoin address (or get it from node info if available)
6. Run `$OCEANLN sign --message "..." --address bc1q...`
7. The CLI will prompt for the mnemonic on stdin -- tell the user to type/paste it
8. Present the base64 signature for the user to paste into OCEAN

When the user asks about balance or wallet operations:

1. Run `$OCEANLN health` to check connectivity
2. Run the appropriate command (`info`, `invoice`, `pay`, `payment`)
3. Use `--json` if structured output is needed for parsing

## Error Handling

- **"invalid mnemonic: expected 24 words, got N"**: Only 24-word BIP39 mnemonics are accepted
- **"only P2WPKH (bc1q...) addresses are supported"**: Pass a native segwit v0 address; legacy and taproot are not supported
- **"invalid BIP32 path"**: Use `m/84'/0'/0'/0/0` style (`'` or `h` for hardened markers)
- **"invalid BOLT12 offer"**: The offer string failed full BOLT12 validation (bech32 + TLV + signature checks)
- **"API (404): ..."**: The Lexe sidecar does not implement the requested endpoint
- **Connection errors**: Lexe sidecar not running at the configured URL

## Architecture Notes

- Single Rust binary; no hardware-wallet support
- The CLI is a stateless client of a **Lexe sidecar** (separately managed by the user; bind address typically `127.0.0.1:5393`)
- BIP-322 signing uses the `bip322` crate (rust-bitcoin) in "simple" mode
- BIP39 → BIP84 derivation: mnemonic → PBKDF2 seed → `Xpriv` → child key at `m/84'/0'/0'/0/0`
- BOLT12 offers are validated via `lightning::offers::offer::Offer` (full structural decode)
- Mnemonic is wrapped in a zero-on-drop type; prompted via `rpassword` with terminal echo disabled
