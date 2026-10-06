//! BIP-322 hash + signing-roundtrip vectors.
//!
//! The tagged-hash vectors are the ones published in BIP-322 itself
//! ("Test vectors" section); they guarantee the `bip322` crate produces
//! the spec's domain-separated `BIP0322-signed-message` hash. A regression
//! here means OCEAN signatures would silently change. The round-trip
//! tests guard the crate's signing primitive and, since 0.0.12, that its
//! verifier binds the witness key to the address.

use bip322::{sign_simple_encoded, tagged_hash, verify_simple_encoded, Verification, BIP322_TAG};

/// BIP-322 test vector: tagged hash of the empty message.
#[test]
fn hash_of_empty_string() {
    let h = tagged_hash(BIP322_TAG, "");
    assert_eq!(
        hex::encode(h),
        "c90c269c4f8fcbe6880f72a721ddfbf1914268a794cbb21cfafee13770ae19f1"
    );
}

/// BIP-322 test vector: tagged hash of "Hello World".
#[test]
fn hash_of_hello_world() {
    let h = tagged_hash(BIP322_TAG, "Hello World");
    assert_eq!(
        hex::encode(h),
        "f0eb03b1a75ac6d9847f55c624a99169b5dccba2a31f5b23bea77ba270de0a7a"
    );
}

/// End-to-end: sign with the example WIF from the upstream `bip322`
/// crate's own example and verify the round-trip. This guards against
/// future breakage in the underlying signing primitive.
#[test]
fn sign_verify_roundtrip() {
    // Upstream example key — corresponds to the P2WPKH address below.
    let wif = "L3VFeEujGtevx9w18HD1fhRbCH67Az2dpCymeRE1SoPK6XQtaN2k";
    let address = "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l";
    let message = "Hello World";

    let sig = sign_simple_encoded(address, message, &[wif], None).expect("sign");
    assert!(matches!(
        verify_simple_encoded(address, message, &sig).expect("verify"),
        Verification::Valid { .. }
    ));
}

/// The verifier must bind the witness pubkey to the address: a signature by
/// `wif` is NOT a signature for an address `wif` does not control, even
/// though the witness itself is internally consistent.
#[test]
fn signature_for_wrong_address_fails_verify() {
    let wif = "L3VFeEujGtevx9w18HD1fhRbCH67Az2dpCymeRE1SoPK6XQtaN2k";
    let own = "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l";
    // BIP-84 vector address — controlled by a different key.
    let other = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu";

    let sig = sign_simple_encoded(own, "Hello World", &[wif], None).expect("sign");
    assert!(verify_simple_encoded(other, "Hello World", &sig).is_err());
}

/// Sanity: a tampered message must NOT verify with the same signature.
#[test]
fn tampered_message_fails_verify() {
    let wif = "L3VFeEujGtevx9w18HD1fhRbCH67Az2dpCymeRE1SoPK6XQtaN2k";
    let address = "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l";

    let sig = sign_simple_encoded(address, "Hello World", &[wif], None).expect("sign");
    assert!(verify_simple_encoded(address, "Hello Mars", &sig).is_err());
}
