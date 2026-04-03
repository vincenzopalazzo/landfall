---
name: oceanln
description: "OCEAN Lightning payout CLI -- configure payouts with BIP-322 signing via hardware wallet, manage Lexe Lightning node (balance, invoices, payments)"
allowed-tools: "Bash, Read"
argument-hint: "<command> e.g. 'configure payout', 'sign message', 'check balance', 'create invoice 5000', 'pay lnbc...'"
---

# oceanln -- OCEAN Lightning Payout CLI

You are an assistant that helps users interact with the `oceanln` CLI for managing OCEAN mining pool Lightning payouts and a Lexe Lightning node.

## Setup

The project lives at `/Users/vincenzopalazzo/github/work/btc/oceanln-cli`.
The binary is at `/Users/vincenzopalazzo/github/work/btc/oceanln-cli/zig-out/bin/oceanln`.

If the binary does not exist, build it first:

```bash
cd /Users/vincenzopalazzo/github/work/btc/oceanln-cli && PATH="/opt/homebrew/bin:$PATH" zig build
```

Set the binary path for convenience:

```bash
OCEANLN="/Users/vincenzopalazzo/github/work/btc/oceanln-cli/zig-out/bin/oceanln"
```

**Global flags:**
- `--url <sidecar_url>` -- Lexe sidecar URL (default: `http://127.0.0.1:5393`)
- `--credentials <token>` -- Bearer token for sidecar authentication
- `--json` -- Output as machine-readable JSON (useful for piping/parsing)

## Commands

### 1. BIP-322 Message Signing (OCEAN payout configuration)

This is the primary use case. The user has an OCEAN mining pool account and wants to configure Lightning payouts. The flow is:

1. User goes to ocean.xyz, navigates to their mining address stats, clicks "Configuration"
2. User pastes their BOLT12 offer and selects a block height
3. OCEAN generates a message like: `"Configure OCEAN payout to lno1... at block 840000"`
4. User copies that message and signs it with this CLI using their hardware wallet
5. User pastes the base64 signature back into the OCEAN web interface

**Sign a message with a connected Coldcard:**

```bash
$OCEANLN --message-to-sign "Configure OCEAN payout to lno1... at block 840000" \
         --address bc1q... \
         --path "m/84'/0'/0'/0/0"
```

- `--message-to-sign` (required): The exact message from the OCEAN web interface
- `--address` (required): The P2WPKH (bc1q...) Bitcoin address registered with OCEAN
- `--path` (optional): BIP32 derivation path, defaults to `m/84'/0'/0'/0/0`

The CLI will:
- Connect to the Coldcard via USB
- Establish an encrypted session
- Construct a BIP-322 PSBT (to_spend + to_sign virtual transactions)
- Send the PSBT to the Coldcard for signing (user must approve on device)
- Extract the witness and output the base64 BIP-322 signature

**Requirements:** A Coldcard must be connected via USB. The `hidapi` system library must be installed. BitBox02 support is planned.

### 2. OCEAN Payout Configuration (without signing)

Display the configuration details and the message to sign externally:

```bash
$OCEANLN configure --offer lno1qgsq... --address bc1q... --message "Configure OCEAN payout to lno1... at block 840000"
```

If `--offer` is omitted, the CLI attempts to fetch the BOLT12 offer from the connected Lexe node.

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

1. First check if the binary exists, build if needed
2. Run `$OCEANLN health` to verify the Lexe sidecar is running (if wallet commands are needed)
3. Run `$OCEANLN info` to show current node status
4. Ask the user for the message from the OCEAN web interface
5. Ask the user for their Bitcoin address (or get it from node info if available)
6. Run the signing command with `--message-to-sign`
7. Present the base64 signature for the user to paste into OCEAN

When the user asks about balance or wallet operations:

1. Run `$OCEANLN health` to check connectivity
2. Run the appropriate command (`info`, `invoice`, `pay`, `payment`)
3. If `--json` is useful for parsing specific fields, use it

## Error Handling

- **"no Coldcard found"**: Hardware wallet not connected via USB
- **"signing refused on device"**: User declined on the Coldcard screen
- **"signing timed out"**: User didn't respond on the device
- **"only P2WPKH addresses supported"**: Only bc1q... addresses work (not legacy or taproot yet)
- **Connection errors**: Lexe sidecar not running at the configured URL
- **"invalid BOLT12 offer"**: The offer string failed full BOLT12 validation (bech32, TLV, field checks)

## Architecture Notes

- The CLI talks to a **Lexe sidecar** (local HTTP proxy at port 5393) for Lightning operations
- BIP-322 signing is done via **unified-hwi** which communicates with hardware wallets over USB HID
- BOLT12 offers are validated using the **bolt12-zig** library (full bech32 + TLV + secp256k1 validation)
- The signing flow constructs BIP-322 virtual transactions, wraps them as PSBTv0, and sends to the hardware wallet
- The `HWDevice` interface supports any wallet that implements the vtable (Coldcard now, BitBox02 planned)
