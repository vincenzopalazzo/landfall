# oceanln

OCEAN Lightning payout CLI. Two commands: generate a BIP39 seed, and configure
an OCEAN payout end-to-end (derive the mining address, create a payable BOLT12
offer on a [Lexe](https://lexe.app) node, and BIP-322 sign the OCEAN message).

## Build

```sh
cargo build --release
```

Binary: `target/release/oceanln`.

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

One command does the whole setup. It prompts for your mnemonic (stdin, echo
disabled), then:

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
SDK and runs the wallet in-process — no separate `lexe-sidecar` needed. `cargo
install --path .` gives you the full CLI, with `init` and `offer`:

```sh
cargo install --path .

oceanln init --generate   # one shot: generate seed + provision wallet + print mining address
oceanln offer --description "OCEAN payout"   # create a payable BOLT12 offer, print it
```

`init --generate` does the whole onboarding at once — generates a fresh 24-word
seed (printed once), derives the **mining address** to register with OCEAN, and
provisions the onchain Lexe wallet. Drop `--generate` to onboard an existing
seed read from stdin. Add `--dry-run` to derive the seed + mining address
**without** provisioning (no network) — handy for testing. `--path` overrides
the address derivation path.

Full OCEAN flow, sidecar-free (the seed is reused across steps, so capture it):

```sh
# Create the wallet and capture the seed + mining address as JSON.
oceanln init --generate --json > wallet.json   # {"mnemonic": "...", "mining_address": "bc1q...", "provisioned": true}
SEED=$(python3 -c 'import sys,json;print(json.load(sys.stdin)["mnemonic"])' < wallet.json)
python3 -c 'import sys,json;print("mining address:", json.load(sys.stdin)["mining_address"])' < wallet.json

# Create the offer, then sign the OCEAN message for it.
OFFER=$(echo "$SEED" | oceanln offer --json --description "OCEAN payout" \
          | python3 -c 'import sys,json;print(json.load(sys.stdin)["offer"])')
# register the mining address + $OFFER on ocean.xyz -> copy the message it gives you
echo "$SEED" | oceanln payout --offer "$OFFER" --message "<exact OCEAN message>"
```

`init` is headless (no app, no Google Drive) — it registers with Lexe's backend
and provisions, exactly like `lexe init` (verified end-to-end on mainnet).

**Thin build:** `cargo build --no-default-features` drops the SDK for a smaller
dependency tree — only `generate` + `payout` (the sidecar client). See issue #3
for the migration notes.

### The Lexe sidecar

`payout` talks to a [Lexe sidecar](https://github.com/lexe-app/lexe-public)
running locally on `127.0.0.1:5393` (or pass `--url`). Launch it with the same
seed `generate` produced:

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
