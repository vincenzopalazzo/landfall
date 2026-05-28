# oceanln

OCEAN Lightning payout CLI. Sign OCEAN payout configuration messages via BIP-322
from a BIP39 mnemonic, and drive a [Lexe](https://lexe.app) Lightning sidecar.

## Build

```sh
cargo build --release
```

Binary: `target/release/oceanln`.

## Use

### Sign an OCEAN configuration message

```sh
oceanln sign \
  --message "Configure OCEAN payout to lno1... at block 840000" \
  --address bc1q...
```

The CLI prompts for your 24-word BIP39 mnemonic on stdin with terminal echo
disabled, derives the BIP84 child key at `m/84'/0'/0'/0/0`, signs via BIP-322
simple mode, and prints the base64 witness for you to paste into the OCEAN web
interface.

The mnemonic is wrapped in a zero-on-drop type — never logged, never written to
disk, never echoed.

### Lexe sidecar commands

Assumes a [Lexe sidecar](https://github.com/lexe-app/lexe-public) is already
running locally on `127.0.0.1:5393` (or pass `--url`). Launch it with:

```sh
lexe-sidecar --client-credentials-path <path-to-your-credentials>
# or set LEXE_CLIENT_CREDENTIALS in your env
```

Then drive it:

```sh
oceanln health
oceanln info
oceanln invoice 5000 "donation"
oceanln pay lnbc50n...
oceanln payment <index>
oceanln configure --message "Configure OCEAN payout to lno1... at block 840000" --offer lno1...
```

Add `--json` to any read command for machine-readable output. Add
`--credentials <token>` to send a `Bearer` header to the sidecar.

`configure` requires `--offer` because the upstream sidecar does not yet
expose a `/v2/node/offer` endpoint — fetch the offer from your node's UI
(or a separate `lncli`/`lightning-cli` session) and pass it in. When the
endpoint lands upstream, the client method is already wired (`SidecarClient::offer`)
so this becomes a follow-up flag change.

If the sidecar isn't running, you'll see:

```
error: could not reach sidecar at http://127.0.0.1:5393 — is `lexe-sidecar` running?
```

## What changed from the Zig version

- Hardware-wallet signing (Coldcard via unified-hwi) was removed; signing is now
  software-only from a BIP39 mnemonic. If you previously relied on the air-gapped
  Coldcard flow, the old Zig binary still works from the `git log` before this
  rewrite.
- `--message-to-sign` was replaced by the `sign` subcommand for cleaner ergonomics.
- Sidecar HTTP client, BIP-322 signing, and BOLT12 offer validation moved from
  hand-rolled Zig to canonical Rust crates: `reqwest`, `bip322`, `bip39`,
  `bitcoin`, `lightning`.

## License

MIT.
