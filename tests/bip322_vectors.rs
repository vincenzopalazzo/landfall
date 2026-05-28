//! BIP-322 hash + signing-roundtrip vectors.
//!
//! These vectors come straight from the previous Zig implementation
//! (`src/bip322.zig:288-307` in the deleted code) to guarantee that
//! the Rust `bip322` crate produces the same domain-separated hash
//! the Zig hand-rolled implementation did. A regression here means
//! OCEAN signatures would silently change.

use bip322::{sign_simple_encoded, tagged_hash, verify_simple_encoded, BIP322_TAG};

/// Test vector lifted from `src/bip322.zig:291`.
#[test]
fn hash_of_empty_string() {
    let h = tagged_hash(BIP322_TAG, "");
    assert_eq!(
        hex::encode(h),
        "c90c269c4f8fcbe6880f72a721ddfbf1914268a794cbb21cfafee13770ae19f1"
    );
}

/// Test vector lifted from `src/bip322.zig:301`.
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

    let sig = sign_simple_encoded(address, message, wif).expect("sign");
    verify_simple_encoded(address, message, &sig).expect("verify");
}

/// Sanity: a tampered message must NOT verify with the same signature.
#[test]
fn tampered_message_fails_verify() {
    let wif = "L3VFeEujGtevx9w18HD1fhRbCH67Az2dpCymeRE1SoPK6XQtaN2k";
    let address = "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l";

    let sig = sign_simple_encoded(address, "Hello World", wif).expect("sign");
    assert!(verify_simple_encoded(address, "Hello Mars", &sig).is_err());
}
