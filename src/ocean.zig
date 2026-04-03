const std = @import("std");

/// Validates that a string looks like a BOLT12 offer (starts with "lno1").
/// TODO: implement proper BOLT12 offer parsing and checksum validation.
/// See: https://github.com/vincenzopalazzo/oceanln-cli/issues/1
pub fn validateBolt12Offer(offer: []const u8) bool {
    // Minimal prefix check — proper bech32m decoding is tracked in #1.
    return offer.len > 4 and std.mem.startsWith(u8, offer, "lno1");
}

/// Validates block height: either "latest" or a numeric string.
pub fn validateBlockHeight(height: []const u8) bool {
    if (std.mem.eql(u8, height, "latest")) return true;
    for (height) |c| {
        if (c < '0' or c > '9') return false;
    }
    return height.len > 0;
}

// ── Tests ────────────────────────────────────────────────────────

test "validateBolt12Offer" {
    try std.testing.expect(validateBolt12Offer("lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9sxw"));
    try std.testing.expect(!validateBolt12Offer("lnbc1234"));
    try std.testing.expect(!validateBolt12Offer(""));
    try std.testing.expect(!validateBolt12Offer("lno"));
}

test "validateBlockHeight" {
    try std.testing.expect(validateBlockHeight("latest"));
    try std.testing.expect(validateBlockHeight("840000"));
    try std.testing.expect(!validateBlockHeight(""));
    try std.testing.expect(!validateBlockHeight("abc"));
    try std.testing.expect(!validateBlockHeight("840 000"));
}
