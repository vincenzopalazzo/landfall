const std = @import("std");

const Allocator = std.mem.Allocator;

/// Constructs the OCEAN payout configuration message.
///
/// The message format is:
///   "Configure OCEAN payout to <bolt12_offer> at block <height>"
///
/// This message must be signed with the private key of the Bitcoin
/// address registered with OCEAN (BIP-322 or legacy message signing).
pub fn buildConfigureMessage(
    allocator: Allocator,
    bolt12_offer: []const u8,
    block_height: []const u8,
) ![]u8 {
    return std.fmt.allocPrint(
        allocator,
        "Configure OCEAN payout to {s} at block {s}",
        .{ bolt12_offer, block_height },
    );
}

/// Validates that a string looks like a BOLT12 offer (starts with "lno1").
pub fn validateBolt12Offer(offer: []const u8) bool {
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

test "buildConfigureMessage" {
    const msg = try buildConfigureMessage(
        std.testing.allocator,
        "lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9sxw",
        "latest",
    );
    defer std.testing.allocator.free(msg);
    try std.testing.expectEqualStrings(
        "Configure OCEAN payout to lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9sxw at block latest",
        msg,
    );
}

test "buildConfigureMessage with numeric height" {
    const msg = try buildConfigureMessage(
        std.testing.allocator,
        "lno1abc",
        "840000",
    );
    defer std.testing.allocator.free(msg);
    try std.testing.expectEqualStrings(
        "Configure OCEAN payout to lno1abc at block 840000",
        msg,
    );
}

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
