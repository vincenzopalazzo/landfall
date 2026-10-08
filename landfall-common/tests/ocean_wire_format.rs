//! Regression suite for the signature wire format OCEAN accepts.
//!
//! Background: the bump to `bip322` 0.0.12 silently changed what
//! `sign_simple_encoded` returns — it started prefixing the base64 witness
//! with `smp` (the crate's own "variant prefix", not part of BIP-322). We
//! shipped that verbatim, OCEAN rejected every Landfall signature with
//! "signature check failed", and our own tests stayed green because the
//! crate's verifier strips the prefix it adds.
//!
//! The lesson: never prove the format with the same library that produced
//! it. Every test here pins the format the way a THIRD PARTY sees it —
//! plain standard base64 → consensus-encoded witness → BIP-322 check with no
//! prefix handling — plus golden vectors (signing is RFC6979-deterministic)
//! so any byte-level change in the output, whatever its cause, fails loudly.

use bip322::{sign_simple_encoded, verify_simple, Verification, SIMPLE_SIGNATURE_PREFIX};
use bitcoin::consensus::Decodable;
use bitcoin::{Address, PrivateKey, Witness};
use landfall_common::sign::{
    address_from_key, derive_private_key, parse_bip32_path, parse_mnemonic, sign_bip322,
    verify_bip322, MnemonicSecret, DEFAULT_BIP32_PATH,
};
use std::str::FromStr;

/// Bitcoin Core's BIP-322 vector key (`src/test/util_tests.cpp`).
const CORE_WIF: &str = "L3VFeEujGtevx9w18HD1fhRbCH67Az2dpCymeRE1SoPK6XQtaN2k";
const CORE_ADDR: &str = "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l";

/// The signature published in BIP-322 for `CORE_ADDR` over "Hello World".
/// This is what a signature pasted into OCEAN looks like: bare base64.
const BIP322_REF_SIG_HELLO_WORLD: &str = "AkcwRAIgZRfIY3p7/DoVTty6YZbWS71bc5Vct9p9Fia83eRmw2QCICK/ENGfwLtptFluMGs2KsqoNSk89pO7F29zJLUx9a/sASECx/EgAxlkQpQ9hYjgGu6EBCPMVPwVIVJqO4XCsMvViHI=";

/// Golden vector: what Landfall must emit for `CORE_WIF` over "Hello World".
/// Signing is deterministic (RFC6979 nonces), so this string is stable across
/// platforms and runs. It differs from `BIP322_REF_SIG_HELLO_WORLD` only in
/// the ECDSA `r` value (Core grinds for low-R, rust-secp256k1 does not); both
/// verify. If this assertion ever fails, the wire format changed — do NOT
/// just update the constant; re-check against OCEAN first.
const LANDFALL_SIG_HELLO_WORLD: &str = "AkgwRQIhAOzyynlqt93lOKJr+wmmxIens//zPzl9tqIOua93wO6MAiBi5n5EyAcPScOjf1lAqIUIQtr3zKNeavYabHyR8eGhowEhAsfxIAMZZEKUPYWI4BruhAQjzFT8FSFSajuFwrDL1Yhy";

/// The CLI smoke-test wallet (`landfall-cli/tests/smoke.rs`), so the golden
/// vector pinned there and the one pinned here come from one derivation.
const TEST_MNEMONIC: &str = "music mystery deliver gospel profit blanket leaf tell photo \
segment letter degree nice plastic duty canyon mammal marble bicycle economy unique find \
cream dune";
const TEST_ADDR: &str = "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r";
const MOCK_OFFER: &str = "lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9uve2rlrm9deu7xyfzrcgqp0s";
const TEST_SIG_PAYOUT_MESSAGE: &str = "AkgwRQIhAK2yBDMcJWwaQKhgSahJXnIhwLMqwnZtynGbWlBicYRyAiBji9sXlanDzE4GI05z6sx5SL2kARxu+J0aB6DX0glhLAEhA5NU2HFyEoSFuz7l3dKjX5f8dIOpaJqL+8FYBEofx8Sq";

fn payout_message() -> String {
    format!("Configure OCEAN payout to {MOCK_OFFER} at block 840000")
}

fn core_key() -> PrivateKey {
    PrivateKey::from_wif(CORE_WIF).unwrap()
}

fn test_wallet_key() -> PrivateKey {
    let m = parse_mnemonic(&MnemonicSecret::from_input(TEST_MNEMONIC)).unwrap();
    let path = parse_bip32_path(DEFAULT_BIP32_PATH).unwrap();
    derive_private_key(&m, &path).unwrap()
}

// ── an OCEAN-side verifier, written without the signing crate's helpers ──

/// Strict RFC 4648 standard-alphabet base64 decoder. Deliberately hand-rolled
/// so this suite does not lean on the `bip322` crate (or its `base64` dep)
/// to decide what counts as base64: a leading `smp`, a URL-safe alphabet, a
/// missing `=` pad — anything OCEAN's decoder would choke on — fails here.
fn strict_base64_decode(s: &str) -> Result<Vec<u8>, String> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = s.as_bytes();
    if bytes.is_empty() || !bytes.len().is_multiple_of(4) {
        return Err(format!("length {} is not a multiple of 4", bytes.len()));
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for (i, chunk) in bytes.chunks(4).enumerate() {
        let mut acc: u32 = 0;
        let mut pad = 0usize;
        for &c in chunk {
            acc <<= 6;
            if c == b'=' {
                pad += 1;
            } else if pad > 0 {
                return Err("data after padding".into());
            } else {
                let v = ALPHABET
                    .iter()
                    .position(|&a| a == c)
                    .ok_or_else(|| format!("byte {c:#04x} ({}) is not base64", c as char))?;
                acc |= v as u32;
            }
        }
        if pad > 2 || (pad > 0 && i != bytes.len() / 4 - 1) {
            return Err("malformed padding".into());
        }
        out.extend_from_slice(&acc.to_be_bytes()[1..4 - pad]);
    }
    Ok(out)
}

/// What OCEAN does with the string a user pastes: decode it as plain base64,
/// parse a consensus-encoded witness, run BIP-322 simple verification for
/// the address. No prefix stripping, no leniency.
fn ocean_accepts(address: &str, message: &str, pasted: &str) -> Result<(), String> {
    let bytes = strict_base64_decode(pasted)?;
    let witness = Witness::consensus_decode(&mut bytes.as_slice())
        .map_err(|e| format!("not a consensus witness: {e}"))?;
    let address = Address::from_str(address).unwrap().assume_checked();
    match verify_simple(&address, message, witness) {
        Ok(Verification::Valid { .. }) => Ok(()),
        Ok(Verification::Inconclusive) => Err("inconclusive".into()),
        Err(e) => Err(format!("invalid: {e}")),
    }
}

/// Shape of a P2WPKH simple-mode signature: a two-item witness
/// (`<der-sig+sighash> <33-byte pubkey>`), which always encodes to a string
/// starting with `Ak` (count byte 0x02, then a 0x47/0x48 length byte).
fn assert_p2wpkh_witness_shape(sig: &str, pubkey: &[u8]) {
    assert!(
        sig.starts_with("Ak"),
        "a 2-item witness always base64-encodes to `Ak…`, got: {sig}"
    );
    let bytes = strict_base64_decode(sig).unwrap_or_else(|e| panic!("{e}: {sig}"));
    let witness = Witness::consensus_decode(&mut bytes.as_slice()).expect("witness");
    assert_eq!(witness.len(), 2, "P2WPKH witness is <sig> <pubkey>");
    let der = witness.nth(0).unwrap();
    assert_eq!(
        *der.last().unwrap(),
        0x01,
        "sighash flag must be SIGHASH_ALL"
    );
    assert_eq!(
        witness.nth(1).unwrap(),
        pubkey,
        "second item is the compressed pubkey"
    );
}

// ── the suite ───────────────────────────────────────────────────

/// The sanity check on the simulated verifier itself: it accepts the
/// signature BIP-322 publishes and rejects it once the message changes.
#[test]
fn ocean_simulation_accepts_the_bip322_reference_signature() {
    ocean_accepts(CORE_ADDR, "Hello World", BIP322_REF_SIG_HELLO_WORLD).unwrap();
    assert!(ocean_accepts(CORE_ADDR, "Hello Mars", BIP322_REF_SIG_HELLO_WORLD).is_err());
}

/// THE regression: a signature straight out of `sign_bip322` must pass the
/// OCEAN-side check with no help from the crate that produced it.
#[test]
fn landfall_signature_passes_an_ocean_side_check() {
    let sig = sign_bip322(&core_key(), CORE_ADDR, "Hello World").unwrap();
    ocean_accepts(CORE_ADDR, "Hello World", &sig)
        .unwrap_or_else(|e| panic!("OCEAN would reject this signature: {e}\n{sig}"));

    let key = test_wallet_key();
    assert_eq!(address_from_key(&key).unwrap(), TEST_ADDR);
    let message = payout_message();
    let sig = sign_bip322(&key, TEST_ADDR, &message).unwrap();
    ocean_accepts(TEST_ADDR, &message, &sig)
        .unwrap_or_else(|e| panic!("OCEAN would reject this signature: {e}\n{sig}"));
}

/// Byte-exact golden vectors. Catches ANY encoding drift — a prefix, a
/// different base64 alphabet, padding, a witness layout change, a sighash
/// flag change — not only the one we have already been bitten by.
#[test]
fn golden_vectors_are_byte_exact() {
    assert_eq!(
        sign_bip322(&core_key(), CORE_ADDR, "Hello World").unwrap(),
        LANDFALL_SIG_HELLO_WORLD
    );
    assert_eq!(
        sign_bip322(&test_wallet_key(), TEST_ADDR, &payout_message()).unwrap(),
        TEST_SIG_PAYOUT_MESSAGE
    );
    // And the golden strings themselves are what OCEAN accepts — so a
    // "fix" that updates both the code and the constants still has to
    // survive the simulated verifier.
    ocean_accepts(CORE_ADDR, "Hello World", LANDFALL_SIG_HELLO_WORLD).unwrap();
    ocean_accepts(TEST_ADDR, &payout_message(), TEST_SIG_PAYOUT_MESSAGE).unwrap();
}

/// Golden vectors only make sense if signing is deterministic. Pin that too,
/// so a switch to randomized nonces (which would silently weaken the vector
/// tests into "whatever we produced today") is caught.
#[test]
fn signing_is_deterministic() {
    let key = core_key();
    let a = sign_bip322(&key, CORE_ADDR, "Hello World").unwrap();
    let b = sign_bip322(&key, CORE_ADDR, "Hello World").unwrap();
    assert_eq!(a, b);
    assert_ne!(a, sign_bip322(&key, CORE_ADDR, "Hello Mars").unwrap());
}

/// The output is bare standard base64 of a two-item P2WPKH witness: no
/// variant prefix, no URL-safe alphabet, proper padding, SIGHASH_ALL, and
/// the signer's own compressed public key as the second item.
#[test]
fn output_is_a_bare_standard_base64_p2wpkh_witness() {
    let secp = bitcoin::secp256k1::Secp256k1::new();
    for (key, addr, msg) in [
        (core_key(), CORE_ADDR, "Hello World".to_string()),
        (test_wallet_key(), TEST_ADDR, payout_message()),
        (test_wallet_key(), TEST_ADDR, String::new()),
        (
            test_wallet_key(),
            TEST_ADDR,
            format!(r#"{{"height":944040,"lightning_bolt12":"{MOCK_OFFER}"}}"#),
        ),
    ] {
        let sig = sign_bip322(&key, addr, &msg).unwrap();
        assert!(
            !sig.starts_with(SIMPLE_SIGNATURE_PREFIX),
            "crate variant prefix leaked into the wire format: {sig}"
        );
        assert!(
            sig.bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'+' || c == b'/' || c == b'='),
            "non-standard base64 alphabet: {sig}"
        );
        assert_p2wpkh_witness_shape(&sig, &key.public_key(&secp).to_bytes());
    }
}

/// Canary on the dependency: `bip322` 0.0.12 *does* prefix its encoded
/// output, and `sign_bip322` is exactly that output minus the prefix. If
/// this fails after a bump, the crate changed its encoding again — go back
/// to `sign_bip322` and re-establish the bare format before trusting any
/// other green test.
#[test]
fn canary_bip322_crate_prefixes_and_sign_bip322_strips_exactly_that() {
    let crate_output = sign_simple_encoded(CORE_ADDR, "Hello World", &[CORE_WIF], None).unwrap();
    assert_eq!(SIMPLE_SIGNATURE_PREFIX, "smp");
    assert!(
        crate_output.starts_with(SIMPLE_SIGNATURE_PREFIX),
        "bip322 changed its encoded output; re-verify the wire format against OCEAN: {crate_output}"
    );
    assert_eq!(
        format!("{SIMPLE_SIGNATURE_PREFIX}{LANDFALL_SIG_HELLO_WORLD}"),
        crate_output
    );
}

/// Our own offline verifier must keep accepting what OCEAN accepts (bare),
/// so `landfall verify` stays a faithful preview of OCEAN's check — and it
/// must still reject a tampered message and the wrong address.
#[test]
fn verify_bip322_agrees_with_the_ocean_side_check() {
    for (addr, msg, sig) in [
        (
            CORE_ADDR,
            "Hello World".to_string(),
            BIP322_REF_SIG_HELLO_WORLD,
        ),
        (
            CORE_ADDR,
            "Hello World".to_string(),
            LANDFALL_SIG_HELLO_WORLD,
        ),
        (TEST_ADDR, payout_message(), TEST_SIG_PAYOUT_MESSAGE),
    ] {
        assert_eq!(
            verify_bip322(addr, &msg, sig).is_ok(),
            ocean_accepts(addr, &msg, sig).is_ok(),
            "verify_bip322 and the OCEAN-side check disagree on {sig}"
        );
        verify_bip322(addr, &msg, sig).unwrap();
        assert!(verify_bip322(addr, &format!("{msg} "), sig).is_err());
    }
    assert!(verify_bip322(TEST_ADDR, "Hello World", LANDFALL_SIG_HELLO_WORLD).is_err());
}
