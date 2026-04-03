//! Integration tests for the oceanln-cli.
//!
//! - BIP-322 signing: MockDevice (software secp256k1 signer) tests the full
//!   pipeline: message hash → PSBT construction → signing → witness extraction
//! - Wallet commands: MockSidecar (HTTP server) tests Lexe client operations:
//!   health, create_invoice, get_payment
const std = @import("std");
const lexe = @import("lexe");
const bitcoin = @import("bitcoin.zig");
const bip322 = @import("bip322.zig");
const signer = @import("signer.zig");
const mock_device = @import("mock_device.zig");
const mock_sidecar = @import("mock_sidecar.zig");

const Sha256 = std.crypto.hash.sha2.Sha256;

// ── Full BIP-322 Signing Pipeline ────────────────────────────────

test "BIP-322 full pipeline: message → PSBT → sign → witness → base64" {
    const allocator = std.testing.allocator;

    // Known test key
    const secret_key = [_]u8{0x01} ** 32;
    var device = mock_device.MockDevice.init(secret_key);
    var hw = device.hwDevice();

    // Build the address scriptPubKey from the mock device's pubkey.
    // For P2WPKH: OP_0 PUSH20 <HASH160(pubkey)>
    // We use SHA256[0..20] as mock HASH160 (same as mock_device.zig).
    var pubkey_sha: [32]u8 = undefined;
    Sha256.hash(&device.compressed_pubkey, &pubkey_sha, .{});
    var spk: [22]u8 = undefined;
    spk[0] = 0x00; // OP_0
    spk[1] = 0x14; // PUSH 20
    @memcpy(spk[2..22], pubkey_sha[0..20]);

    const message = "Configure OCEAN payout to lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9sxw at block 840000";

    // Step 1: Compute message hash
    const msg_hash = bip322.messageHash(message);

    // Step 2: Build to_spend transaction
    var to_spend_data = bip322.buildToSpend(msg_hash, &spk);
    const to_spend_raw = try bitcoin.serializeTx(allocator, to_spend_data.tx());
    defer allocator.free(to_spend_raw);
    try std.testing.expect(to_spend_raw.len > 0);

    // Step 3: Compute to_spend txid and build to_sign
    const to_spend_txid = try bitcoin.txid(allocator, to_spend_data.tx());
    var to_sign_data = bip322.buildToSign(to_spend_txid);

    // Step 4: Wrap as PSBTv0
    const fp = try hw.getMasterFingerprint(allocator);
    const path = [_]u32{ 84 | 0x80000000, 0 | 0x80000000, 0 | 0x80000000, 0, 0 };

    const psbt = try bip322.buildPsbtV0(
        allocator,
        to_sign_data.tx(),
        to_spend_data.output[0],
        &path,
        fp,
        device.compressed_pubkey,
    );
    defer allocator.free(psbt);

    // Verify PSBT structure
    try std.testing.expectEqualStrings("psbt", psbt[0..4]);
    try std.testing.expectEqual(@as(u8, 0xff), psbt[4]);

    // Step 5: Sign with mock device
    const signed_psbt = try hw.signTx(allocator, psbt);
    defer allocator.free(signed_psbt);
    try std.testing.expect(signed_psbt.len > psbt.len); // should be larger with witness

    // Step 6: Extract witness
    const witness = try bip322.extractWitness(signed_psbt);
    try std.testing.expect(witness.len > 0);
    try std.testing.expectEqual(@as(u8, 0x02), witness[0]); // 2 stack items

    // Step 7: Base64 encode
    const signature = try bip322.encodeSignature(allocator, witness);
    defer allocator.free(signature);
    try std.testing.expect(signature.len > 0);

    // Verify it's valid base64
    const decoded_len = std.base64.standard.Decoder.calcSizeForSlice(signature) catch unreachable;
    try std.testing.expect(decoded_len > 0);
}

test "BIP-322 signBip322WithDevice end-to-end" {
    const allocator = std.testing.allocator;

    const secret_key = [_]u8{0x01} ** 32;
    var device = mock_device.MockDevice.init(secret_key);

    // Build a bc1q-style address string for the mock device.
    // For this test, we use a well-known address and construct the PSBT
    // with a matching scriptPubKey. The mock signer doesn't verify the
    // address matches the key — it just signs whatever PSBT it receives.
    //
    // For a real integration test with address verification, we'd need
    // to derive the bc1q address from the mock device's pubkey.
    // For now, use bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4 (a standard test vector).
    const address = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";
    const path = [_]u32{ 84 | 0x80000000, 0 | 0x80000000, 0 | 0x80000000, 0, 0 };

    const signature = try signer.signBip322WithDevice(
        allocator,
        device.hwDevice(),
        "test message for OCEAN",
        address,
        &path,
    );
    defer allocator.free(signature);

    // Should produce a non-empty base64 string
    try std.testing.expect(signature.len > 10);

    // Decode base64 to verify it's valid
    const decoded_len = std.base64.standard.Decoder.calcSizeForSlice(signature) catch unreachable;
    const decoded = try allocator.alloc(u8, decoded_len);
    defer allocator.free(decoded);
    std.base64.standard.Decoder.decode(decoded, signature) catch unreachable;

    // First byte of decoded witness should be 0x02 (2 stack items)
    try std.testing.expectEqual(@as(u8, 0x02), decoded[0]);
}

test "BIP-322 deterministic: same input produces same signature" {
    const allocator = std.testing.allocator;
    const secret_key = [_]u8{0x02} ** 32;

    var device1 = mock_device.MockDevice.init(secret_key);
    var device2 = mock_device.MockDevice.init(secret_key);

    const address = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";
    const path = [_]u32{ 84 | 0x80000000, 0 | 0x80000000, 0 | 0x80000000, 0, 0 };
    const message = "deterministic test";

    const sig1 = try signer.signBip322WithDevice(allocator, device1.hwDevice(), message, address, &path);
    defer allocator.free(sig1);

    const sig2 = try signer.signBip322WithDevice(allocator, device2.hwDevice(), message, address, &path);
    defer allocator.free(sig2);

    try std.testing.expectEqualStrings(sig1, sig2);
}

test "BIP-322 different messages produce different signatures" {
    const allocator = std.testing.allocator;
    const secret_key = [_]u8{0x03} ** 32;

    var device1 = mock_device.MockDevice.init(secret_key);
    var device2 = mock_device.MockDevice.init(secret_key);

    const address = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";
    const path = [_]u32{ 84 | 0x80000000, 0 | 0x80000000, 0 | 0x80000000, 0, 0 };

    const sig1 = try signer.signBip322WithDevice(allocator, device1.hwDevice(), "message A", address, &path);
    defer allocator.free(sig1);

    const sig2 = try signer.signBip322WithDevice(allocator, device2.hwDevice(), "message B", address, &path);
    defer allocator.free(sig2);

    try std.testing.expect(!std.mem.eql(u8, sig1, sig2));
}

test "BIP-322 different keys produce different signatures" {
    const allocator = std.testing.allocator;

    var device1 = mock_device.MockDevice.init([_]u8{0x04} ** 32);
    var device2 = mock_device.MockDevice.init([_]u8{0x05} ** 32);

    const address = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";
    const path = [_]u32{ 84 | 0x80000000, 0 | 0x80000000, 0 | 0x80000000, 0, 0 };
    const message = "same message different keys";

    const sig1 = try signer.signBip322WithDevice(allocator, device1.hwDevice(), message, address, &path);
    defer allocator.free(sig1);

    const sig2 = try signer.signBip322WithDevice(allocator, device2.hwDevice(), message, address, &path);
    defer allocator.free(sig2);

    try std.testing.expect(!std.mem.eql(u8, sig1, sig2));
}

// ── Wallet Command Integration Tests ─────────────────────────────

const TestCtx = struct {
    sidecar: mock_sidecar.MockSidecar,
    client: lexe.LexeClient,

    fn deinit(self: *TestCtx) void {
        self.client.deinit();
        self.sidecar.stop();
    }
};

// Use a fixed URL since the port is always 15393
const mock_url = "http://127.0.0.1:15393";

fn startMockAndClient(allocator: std.mem.Allocator) !TestCtx {
    var sidecar = mock_sidecar.MockSidecar.init(15393);
    try sidecar.start();
    std.Thread.sleep(10_000_000); // 10ms for server to bind

    const client = lexe.LexeClient.init(allocator, .{
        .base_url = mock_url,
    }) catch return error.ClientInitFailed;

    return .{ .sidecar = sidecar, .client = client };
}

test "wallet: health check via mock sidecar" {
    const allocator = std.testing.allocator;
    var ctx = try startMockAndClient(allocator);
    defer ctx.deinit();

    const result = try ctx.client.health();
    switch (result) {
        .ok => |resp| {
            defer resp.deinit();
            try std.testing.expectEqualStrings("ok", resp.value.status);
        },
        .err => |resp| {
            defer resp.deinit();
            return error.UnexpectedApiError;
        },
        .not_found => return error.UnexpectedNotFound,
    }
}

test "wallet: node info via mock sidecar" {
    const allocator = std.testing.allocator;
    var ctx = try startMockAndClient(allocator);
    defer ctx.deinit();

    const result = try ctx.client.nodeInfo();
    switch (result) {
        .ok => |resp| {
            defer resp.deinit();
            const info = resp.value;
            try std.testing.expectEqualStrings("0.9.2-mock", info.version);
            try std.testing.expectEqualStrings("50000", info.balance);
            try std.testing.expectEqualStrings("25000", info.lightning_balance);
            try std.testing.expectEqual(@as(u32, 2), info.num_channels);
            try std.testing.expectEqual(@as(u32, 2), info.num_usable_channels);
        },
        .err => |resp| {
            defer resp.deinit();
            return error.UnexpectedApiError;
        },
        .not_found => return error.UnexpectedNotFound,
    }
}

test "wallet: create invoice via mock sidecar" {
    const allocator = std.testing.allocator;
    var ctx = try startMockAndClient(allocator);
    defer ctx.deinit();

    const result = try ctx.client.createInvoice(.{
        .amount = "1000",
        .description = "test invoice",
        .expiration_secs = 3600,
    });
    switch (result) {
        .ok => |resp| {
            defer resp.deinit();
            const inv = resp.value;
            try std.testing.expectEqualStrings("1000", inv.amount.?);
            try std.testing.expectEqualStrings("mock invoice", inv.description.?);
            try std.testing.expect(std.mem.startsWith(u8, inv.invoice, "lnbc"));
            try std.testing.expect(inv.payment_hash.len > 0);
            try std.testing.expect(inv.expires_at > inv.created_at);
        },
        .err => |resp| {
            defer resp.deinit();
            return error.UnexpectedApiError;
        },
        .not_found => return error.UnexpectedNotFound,
    }
}

test "wallet: get payment via mock sidecar" {
    const allocator = std.testing.allocator;
    var ctx = try startMockAndClient(allocator);
    defer ctx.deinit();

    const result = try ctx.client.getPayment("0000001772349163844-ln_mock");
    switch (result) {
        .ok => |resp| {
            defer resp.deinit();
            const p = resp.value;
            try std.testing.expectEqualStrings("0000001772349163844-ln_mock", p.index);
            try std.testing.expectEqual(lexe.PaymentRail.invoice, p.rail);
            try std.testing.expectEqual(lexe.PaymentKind.invoice, p.kind);
            try std.testing.expectEqual(lexe.PaymentDirection.inbound, p.direction);
            try std.testing.expectEqual(lexe.PaymentStatus.completed, p.status);
            try std.testing.expectEqualStrings("1000", p.amount.?);
            try std.testing.expectEqualStrings("0", p.fees);
            try std.testing.expectEqualStrings("received", p.status_msg);
        },
        .err => |resp| {
            defer resp.deinit();
            return error.UnexpectedApiError;
        },
        .not_found => return error.UnexpectedNotFound,
    }
}
