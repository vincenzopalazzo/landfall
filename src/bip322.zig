const std = @import("std");
const Sha256 = std.crypto.hash.sha2.Sha256;
const bitcoin = @import("bitcoin.zig");
const Allocator = std.mem.Allocator;

// ── Tagged Hash ──────────────────────────────────────────────────

/// BIP-340 tagged hash: SHA256(SHA256(tag) || SHA256(tag) || msg)
pub fn taggedHash(tag: []const u8, msg: []const u8) [32]u8 {
    var tag_hash: [32]u8 = undefined;
    Sha256.hash(tag, &tag_hash, .{});

    var h = Sha256.init(.{});
    h.update(&tag_hash);
    h.update(&tag_hash);
    h.update(msg);
    var result: [32]u8 = undefined;
    h.final(&result);
    return result;
}

/// BIP-322 message hash: taggedHash("BIP0322-signed-message", message)
pub fn messageHash(message: []const u8) [32]u8 {
    return taggedHash("BIP0322-signed-message", message);
}

// ── BIP-322 Transaction Construction ─────────────────────────────

/// Build the BIP-322 "to_spend" virtual transaction.
///
/// nVersion=0, nLockTime=0
/// vin[0]: prevout=00..00:FFFFFFFF, scriptSig=OP_0 PUSH32[msg_hash], seq=0
/// vout[0]: value=0, scriptPubKey=<challenge_script>
/// Stable storage for a BIP-322 to_spend transaction.
/// All slices in the Transaction point into this struct's fields.
pub const ToSpendTx = struct {
    script_sig: [34]u8,
    input: [1]bitcoin.TxInput,
    output: [1]bitcoin.TxOutput,

    pub fn tx(self: *ToSpendTx) bitcoin.Transaction {
        return .{
            .version = 0,
            .inputs = &self.input,
            .outputs = &self.output,
            .locktime = 0,
        };
    }
};

/// Build the BIP-322 "to_spend" virtual transaction.
pub fn buildToSpend(msg_hash: [32]u8, script_pubkey: []const u8) ToSpendTx {
    var result: ToSpendTx = undefined;
    result.script_sig[0] = 0x00; // OP_0
    result.script_sig[1] = 0x20; // OP_PUSHBYTES_32
    @memcpy(result.script_sig[2..34], &msg_hash);
    result.input[0] = .{
        .prevout = .{ .txid = [_]u8{0} ** 32, .vout = 0xffffffff },
        .script_sig = &result.script_sig,
        .sequence = 0,
    };
    result.output[0] = .{ .value = 0, .script_pubkey = script_pubkey };
    return result;
}

/// Stable storage for a BIP-322 to_sign transaction.
pub const ToSignTx = struct {
    input: [1]bitcoin.TxInput,
    output: [1]bitcoin.TxOutput,
    op_return: [1]u8,

    pub fn tx(self: *ToSignTx) bitcoin.Transaction {
        return .{
            .version = 0,
            .inputs = &self.input,
            .outputs = &self.output,
            .locktime = 0,
        };
    }
};

/// Build the BIP-322 "to_sign" transaction.
pub fn buildToSign(to_spend_txid: [32]u8) ToSignTx {
    var result: ToSignTx = undefined;
    result.op_return[0] = 0x6a; // OP_RETURN
    result.input[0] = .{
        .prevout = .{ .txid = to_spend_txid, .vout = 0 },
        .script_sig = &[_]u8{},
        .sequence = 0,
    };
    result.output[0] = .{ .value = 0, .script_pubkey = &result.op_return };
    return result;
}

// ── PSBT v0 Construction ─────────────────────────────────────────

const PSBT_MAGIC = "psbt\xff";

/// Build a minimal PSBTv0 wrapping the to_sign transaction.
///
/// Global: PSBT_GLOBAL_UNSIGNED_TX (0x00) = serialized to_sign
/// Input 0:
///   PSBT_IN_WITNESS_UTXO (0x01) = serialized to_spend vout[0]
///   PSBT_IN_BIP32_DERIVATION (0x06 || 33-byte pubkey) = fingerprint + path
/// Output 0: (separator only)
pub fn buildPsbtV0(
    allocator: Allocator,
    to_sign: bitcoin.Transaction,
    witness_utxo: bitcoin.TxOutput,
    path: []const u32,
    master_fp: [4]u8,
    pubkey: [33]u8,
) ![]u8 {
    // Serialize the unsigned tx
    const unsigned_tx = try bitcoin.serializeTx(allocator, to_sign);
    defer allocator.free(unsigned_tx);

    // Serialize the witness UTXO (just the TxOut: value + scriptPubKey)
    const utxo_len = 8 + bitcoin.compactSizeLen(witness_utxo.script_pubkey.len) + witness_utxo.script_pubkey.len;
    const utxo_buf = try allocator.alloc(u8, utxo_len);
    defer allocator.free(utxo_buf);
    {
        var pos: usize = 0;
        std.mem.writeInt(i64, utxo_buf[0..8], witness_utxo.value, .little);
        pos = 8;
        bitcoin.writeCompactSize(utxo_buf, &pos, witness_utxo.script_pubkey.len);
        @memcpy(utxo_buf[pos..][0..witness_utxo.script_pubkey.len], witness_utxo.script_pubkey);
    }

    // BIP32 derivation value: 4-byte fingerprint + N * 4-byte path elements
    const bip32_val_len = 4 + path.len * 4;

    // Calculate total PSBT size
    var total: usize = 0;
    total += PSBT_MAGIC.len; // magic
    // Global: key 0x00, value = unsigned_tx
    total += 1 + bitcoin.compactSizeLen(unsigned_tx.len) + unsigned_tx.len; // key-len(1) + key(1=0x00) is actually: compact_size(key_len=1) + key(0x00) + compact_size(value_len) + value
    // Actually PSBT format: compact_size(key_len) + key_bytes + compact_size(value_len) + value_bytes
    // For global unsigned tx: key = [0x00], so key_len = 1

    // Let me recalculate properly
    total = 0;
    total += PSBT_MAGIC.len;

    // Global map
    total += compactKvLen(1, unsigned_tx.len); // PSBT_GLOBAL_UNSIGNED_TX
    total += 1; // separator 0x00

    // Input 0 map
    total += compactKvLen(1, utxo_len); // PSBT_IN_WITNESS_UTXO (key = [0x01])
    total += compactKvLen(1 + 33, bip32_val_len); // PSBT_IN_BIP32_DERIVATION (key = [0x06] + pubkey)
    total += 1; // separator 0x00

    // Output 0 map
    total += 1; // separator 0x00

    const psbt = try allocator.alloc(u8, total);
    errdefer allocator.free(psbt);
    var pos: usize = 0;

    // Magic
    @memcpy(psbt[pos..][0..PSBT_MAGIC.len], PSBT_MAGIC);
    pos += PSBT_MAGIC.len;

    // Global: PSBT_GLOBAL_UNSIGNED_TX
    writeKv(psbt, &pos, &[_]u8{0x00}, unsigned_tx);
    psbt[pos] = 0x00; // separator
    pos += 1;

    // Input 0: PSBT_IN_WITNESS_UTXO
    writeKv(psbt, &pos, &[_]u8{0x01}, utxo_buf);

    // Input 0: PSBT_IN_BIP32_DERIVATION
    var bip32_key: [34]u8 = undefined;
    bip32_key[0] = 0x06;
    @memcpy(bip32_key[1..34], &pubkey);
    var bip32_val_buf: [4 + 12 * 4]u8 = undefined; // max 12 path components
    @memcpy(bip32_val_buf[0..4], &master_fp);
    for (path, 0..) |component, idx| {
        std.mem.writeInt(u32, bip32_val_buf[4 + idx * 4 ..][0..4], component, .little);
    }
    writeKv(psbt, &pos, &bip32_key, bip32_val_buf[0..bip32_val_len]);

    psbt[pos] = 0x00; // input separator
    pos += 1;

    psbt[pos] = 0x00; // output separator
    pos += 1;

    std.debug.assert(pos == total);
    return psbt;
}

fn compactKvLen(key_len: usize, value_len: usize) usize {
    return bitcoin.compactSizeLen(key_len) + key_len +
        bitcoin.compactSizeLen(value_len) + value_len;
}

fn writeKv(buf: []u8, pos: *usize, key: []const u8, value: []const u8) void {
    bitcoin.writeCompactSize(buf, pos, key.len);
    @memcpy(buf[pos.*..][0..key.len], key);
    pos.* += key.len;
    bitcoin.writeCompactSize(buf, pos, value.len);
    @memcpy(buf[pos.*..][0..value.len], value);
    pos.* += value.len;
}

// ── PSBT Witness Extraction ──────────────────────────────────────

pub const PsbtError = error{
    InvalidMagic,
    InvalidFormat,
    NoWitness,
};

/// Read a compact size from a byte slice at the given offset.
fn readCompactSize(data: []const u8, pos: *usize) PsbtError!u64 {
    if (pos.* >= data.len) return error.InvalidFormat;
    const first = data[pos.*];
    pos.* += 1;
    if (first < 0xfd) return first;
    if (first == 0xfd) {
        if (pos.* + 2 > data.len) return error.InvalidFormat;
        const val = std.mem.readInt(u16, data[pos.*..][0..2], .little);
        pos.* += 2;
        return val;
    }
    if (first == 0xfe) {
        if (pos.* + 4 > data.len) return error.InvalidFormat;
        const val = std.mem.readInt(u32, data[pos.*..][0..4], .little);
        pos.* += 4;
        return val;
    }
    if (pos.* + 8 > data.len) return error.InvalidFormat;
    const val = std.mem.readInt(u64, data[pos.*..][0..8], .little);
    pos.* += 8;
    return val;
}

/// Extract the PSBT_IN_FINAL_SCRIPTWITNESS (key type 0x07) from a signed PSBT.
/// Returns the raw witness bytes (the value of key 0x07 in input 0).
pub fn extractWitness(signed_psbt: []const u8) PsbtError![]const u8 {
    if (signed_psbt.len < PSBT_MAGIC.len) return error.InvalidMagic;
    if (!std.mem.eql(u8, signed_psbt[0..PSBT_MAGIC.len], PSBT_MAGIC)) return error.InvalidMagic;

    var pos: usize = PSBT_MAGIC.len;

    // Skip global map (read key-value pairs until separator 0x00)
    while (pos < signed_psbt.len) {
        const key_len = try readCompactSize(signed_psbt, &pos);
        if (key_len == 0) break; // separator
        if (pos + key_len > signed_psbt.len) return error.InvalidFormat;
        pos += @intCast(key_len); // skip key
        const val_len = try readCompactSize(signed_psbt, &pos);
        if (pos + val_len > signed_psbt.len) return error.InvalidFormat;
        pos += @intCast(val_len); // skip value
    }

    // Now in input 0 map — look for key type 0x07
    while (pos < signed_psbt.len) {
        const key_len = try readCompactSize(signed_psbt, &pos);
        if (key_len == 0) break; // separator
        if (pos + key_len > signed_psbt.len) return error.InvalidFormat;
        const key_type = signed_psbt[pos];
        pos += @intCast(key_len); // skip key
        const val_len = try readCompactSize(signed_psbt, &pos);
        if (pos + val_len > signed_psbt.len) return error.InvalidFormat;
        if (key_type == 0x07) {
            // PSBT_IN_FINAL_SCRIPTWITNESS
            return signed_psbt[pos .. pos + @as(usize, @intCast(val_len))];
        }
        pos += @intCast(val_len); // skip value
    }

    return error.NoWitness;
}

/// Base64-encode witness bytes for BIP-322 "simple" signature output.
pub fn encodeSignature(allocator: Allocator, witness: []const u8) ![]u8 {
    const encoded_len = std.base64.standard.Encoder.calcSize(witness.len);
    const buf = try allocator.alloc(u8, encoded_len);
    _ = std.base64.standard.Encoder.encode(buf, witness);
    return buf;
}

// ── Tests ────────────────────────────────────────────────────────

test "messageHash empty string" {
    const hash = messageHash("");
    // Known BIP-322 test vector for empty message
    const expected = "c90c269c4f8fcbe6880f72a721ddfbf1914268a794cbb21cfafee13770ae19f1";
    var hex: [64]u8 = undefined;
    for (hash, 0..) |b, idx| {
        _ = std.fmt.bufPrint(hex[idx * 2 ..][0..2], "{x:0>2}", .{b}) catch unreachable;
    }
    try std.testing.expectEqualStrings(expected, &hex);
}

test "messageHash Hello World" {
    const hash = messageHash("Hello World");
    const expected = "f0eb03b1a75ac6d9847f55c624a99169b5dccba2a31f5b23bea77ba270de0a7a";
    var hex: [64]u8 = undefined;
    for (hash, 0..) |b, idx| {
        _ = std.fmt.bufPrint(hex[idx * 2 ..][0..2], "{x:0>2}", .{b}) catch unreachable;
    }
    try std.testing.expectEqualStrings(expected, &hex);
}

test "buildPsbtV0 starts with magic" {
    const msg_hash = messageHash("test");
    const spk = [_]u8{ 0x00, 0x14 } ++ [_]u8{0xaa} ** 20;
    var to_spend_data = buildToSpend(msg_hash, &spk);
    const to_spend_id = try bitcoin.txid(std.testing.allocator, to_spend_data.tx());
    var to_sign_data = buildToSign(to_spend_id);

    const psbt = try buildPsbtV0(
        std.testing.allocator,
        to_sign_data.tx(),
        to_spend_data.output[0],
        &[_]u32{ 84 | 0x80000000, 0 | 0x80000000, 0 | 0x80000000, 0, 0 },
        [_]u8{ 0x01, 0x02, 0x03, 0x04 },
        [_]u8{0x02} ++ [_]u8{0xbb} ** 32,
    );
    defer std.testing.allocator.free(psbt);

    try std.testing.expect(psbt.len > 5);
    try std.testing.expectEqualStrings("psbt", psbt[0..4]);
    try std.testing.expectEqual(@as(u8, 0xff), psbt[4]);
}
