const std = @import("std");
const bolt12 = @import("bolt12");

/// Validate a BOLT12 offer string by fully decoding it.
/// Uses the bolt12-zig library for bech32 decoding, TLV parsing,
/// and offer field validation (including point-on-curve checks).
pub fn validateBolt12Offer(allocator: std.mem.Allocator, offer_str: []const u8) bool {
    const decoded = bolt12.decodeOffer(allocator, offer_str) catch return false;
    decoded.deinit();
    return true;
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

test "validateBolt12Offer rejects garbage" {
    try std.testing.expect(!validateBolt12Offer(std.testing.allocator, "lnbc1234"));
    try std.testing.expect(!validateBolt12Offer(std.testing.allocator, ""));
    try std.testing.expect(!validateBolt12Offer(std.testing.allocator, "lno"));
    try std.testing.expect(!validateBolt12Offer(std.testing.allocator, "not-an-offer"));
}

test "validateBlockHeight" {
    try std.testing.expect(validateBlockHeight("latest"));
    try std.testing.expect(validateBlockHeight("840000"));
    try std.testing.expect(!validateBlockHeight(""));
    try std.testing.expect(!validateBlockHeight("abc"));
    try std.testing.expect(!validateBlockHeight("840 000"));
}
