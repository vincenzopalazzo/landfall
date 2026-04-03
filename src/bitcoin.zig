const std = @import("std");
const Sha256 = std.crypto.hash.sha2.Sha256;
const Allocator = std.mem.Allocator;

// ── Types ────────────────────────────────────────────────────────

pub const TxOutpoint = struct {
    txid: [32]u8,
    vout: u32,
};

pub const TxInput = struct {
    prevout: TxOutpoint,
    script_sig: []const u8,
    sequence: u32,
};

pub const TxOutput = struct {
    value: i64,
    script_pubkey: []const u8,
};

pub const Transaction = struct {
    version: i32,
    inputs: []const TxInput,
    outputs: []const TxOutput,
    locktime: u32,
};

// ── Hashing ──────────────────────────────────────────────────────

pub fn doubleSha256(data: []const u8) [32]u8 {
    var first: [32]u8 = undefined;
    Sha256.hash(data, &first, .{});
    var second: [32]u8 = undefined;
    Sha256.hash(&first, &second, .{});
    return second;
}

// ── Compact Size ─────────────────────────────────────────────────

pub fn writeCompactSize(buf: []u8, pos: *usize, value: u64) void {
    if (value < 0xfd) {
        buf[pos.*] = @intCast(value);
        pos.* += 1;
    } else if (value <= 0xffff) {
        buf[pos.*] = 0xfd;
        std.mem.writeInt(u16, buf[pos.* + 1 ..][0..2], @intCast(value), .little);
        pos.* += 3;
    } else if (value <= 0xffffffff) {
        buf[pos.*] = 0xfe;
        std.mem.writeInt(u32, buf[pos.* + 1 ..][0..4], @intCast(value), .little);
        pos.* += 5;
    } else {
        buf[pos.*] = 0xff;
        std.mem.writeInt(u64, buf[pos.* + 1 ..][0..8], value, .little);
        pos.* += 9;
    }
}

pub fn compactSizeLen(value: u64) usize {
    if (value < 0xfd) return 1;
    if (value <= 0xffff) return 3;
    if (value <= 0xffffffff) return 5;
    return 9;
}

// ── Transaction Serialization ────────────────────────────────────

pub fn serializedTxLen(tx: Transaction) usize {
    var len: usize = 4; // version
    len += compactSizeLen(tx.inputs.len);
    for (tx.inputs) |inp| {
        len += 32 + 4; // prevout
        len += compactSizeLen(inp.script_sig.len);
        len += inp.script_sig.len;
        len += 4; // sequence
    }
    len += compactSizeLen(tx.outputs.len);
    for (tx.outputs) |out| {
        len += 8; // value
        len += compactSizeLen(out.script_pubkey.len);
        len += out.script_pubkey.len;
    }
    len += 4; // locktime
    return len;
}

pub fn serializeTx(allocator: Allocator, tx: Transaction) ![]u8 {
    const total = serializedTxLen(tx);
    const buf = try allocator.alloc(u8, total);
    errdefer allocator.free(buf);
    var pos: usize = 0;

    // version
    std.mem.writeInt(i32, buf[pos..][0..4], tx.version, .little);
    pos += 4;

    // inputs
    writeCompactSize(buf, &pos, tx.inputs.len);
    for (tx.inputs) |inp| {
        @memcpy(buf[pos..][0..32], &inp.prevout.txid);
        pos += 32;
        std.mem.writeInt(u32, buf[pos..][0..4], inp.prevout.vout, .little);
        pos += 4;
        writeCompactSize(buf, &pos, inp.script_sig.len);
        @memcpy(buf[pos..][0..inp.script_sig.len], inp.script_sig);
        pos += inp.script_sig.len;
        std.mem.writeInt(u32, buf[pos..][0..4], inp.sequence, .little);
        pos += 4;
    }

    // outputs
    writeCompactSize(buf, &pos, tx.outputs.len);
    for (tx.outputs) |out| {
        std.mem.writeInt(i64, buf[pos..][0..8], out.value, .little);
        pos += 8;
        writeCompactSize(buf, &pos, out.script_pubkey.len);
        @memcpy(buf[pos..][0..out.script_pubkey.len], out.script_pubkey);
        pos += out.script_pubkey.len;
    }

    // locktime
    std.mem.writeInt(u32, buf[pos..][0..4], tx.locktime, .little);
    pos += 4;

    std.debug.assert(pos == total);
    return buf;
}

pub fn txid(allocator: Allocator, tx: Transaction) ![32]u8 {
    const raw = try serializeTx(allocator, tx);
    defer allocator.free(raw);
    return doubleSha256(raw);
}

// ── Bech32 Decoding (P2WPKH only) ───────────────────────────────

const bech32_charset = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";

fn bech32CharValue(ch: u8) ?u5 {
    return for (bech32_charset, 0..) |c, i| {
        if (c == ch) break @intCast(i);
    } else null;
}

fn bech32Polymod(values: []const u8) u32 {
    const gen = [5]u32{ 0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3 };
    var chk: u32 = 1;
    for (values) |v| {
        const top = chk >> 25;
        chk = ((chk & 0x1ffffff) << 5) ^ v;
        for (gen, 0..) |g, i| {
            if ((top >> @intCast(i)) & 1 == 1) chk ^= g;
        }
    }
    return chk;
}

fn bech32HrpExpand(hrp: []const u8, buf: []u8) usize {
    var pos: usize = 0;
    for (hrp) |c| {
        buf[pos] = c >> 5;
        pos += 1;
    }
    buf[pos] = 0;
    pos += 1;
    for (hrp) |c| {
        buf[pos] = c & 0x1f;
        pos += 1;
    }
    return pos;
}

pub const Bech32Error = error{
    InvalidChar,
    InvalidChecksum,
    NoSeparator,
    InvalidWitnessVersion,
    InvalidProgramLen,
    NotP2wpkh,
};

pub const Bech32Result = struct {
    witness_version: u8,
    witness_program: [20]u8,
};

pub fn bech32Decode(addr: []const u8) Bech32Error!Bech32Result {
    // Find separator
    var sep_pos: usize = 0;
    var found = false;
    var i = addr.len;
    while (i > 0) {
        i -= 1;
        if (addr[i] == '1') {
            sep_pos = i;
            found = true;
            break;
        }
    }
    if (!found) return error.NoSeparator;

    const hrp = addr[0..sep_pos];
    const data_part = addr[sep_pos + 1 ..];

    // Decode data characters to 5-bit values
    var data5: [128]u8 = undefined;
    if (data_part.len > data5.len) return error.InvalidChar;
    for (data_part, 0..) |ch, idx| {
        const lower = if (ch >= 'A' and ch <= 'Z') ch + 32 else ch;
        data5[idx] = bech32CharValue(lower) orelse return error.InvalidChar;
    }
    const data = data5[0..data_part.len];

    // Verify checksum
    var check_buf: [256]u8 = undefined;
    const hrp_len = bech32HrpExpand(hrp, &check_buf);
    @memcpy(check_buf[hrp_len..][0..data.len], data);
    const polymod = bech32Polymod(check_buf[0 .. hrp_len + data.len]);
    // Accept bech32 (polymod==1) or bech32m (polymod==0x2bc830a3)
    if (polymod != 1 and polymod != 0x2bc830a3) return error.InvalidChecksum;

    // Strip 6-char checksum
    if (data.len < 7) return error.InvalidProgramLen;
    const payload = data[0 .. data.len - 6];

    const witness_version = payload[0];
    if (witness_version != 0) return error.InvalidWitnessVersion; // v1: P2WPKH only

    // Convert 5-bit groups to 8-bit
    const groups = payload[1..];
    var result: Bech32Result = .{
        .witness_version = witness_version,
        .witness_program = undefined,
    };

    var acc: u32 = 0;
    var bits: u5 = 0;
    var out_idx: usize = 0;
    for (groups) |val| {
        acc = (acc << 5) | val;
        bits += 5;
        if (bits >= 8) {
            bits -= 8;
            if (out_idx >= 20) return error.InvalidProgramLen;
            result.witness_program[out_idx] = @intCast((acc >> bits) & 0xff);
            out_idx += 1;
        }
    }
    if (out_idx != 20) return error.InvalidProgramLen;

    return result;
}

/// Convert a bc1q... address to its 22-byte P2WPKH scriptPubKey:
/// OP_0 (0x00) + PUSH20 (0x14) + 20-byte witness program
pub fn scriptPubkeyFromAddress(addr: []const u8) Bech32Error![22]u8 {
    const decoded = try bech32Decode(addr);
    var spk: [22]u8 = undefined;
    spk[0] = 0x00; // OP_0
    spk[1] = 0x14; // PUSH 20 bytes
    @memcpy(spk[2..22], &decoded.witness_program);
    return spk;
}

// ── Base58 Check Decode (for xpub parsing) ───────────────────────

const base58_alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

pub const Base58Error = error{
    InvalidChar,
    InvalidChecksum,
    TooShort,
};

/// Decode a base58check string and return the payload (without checksum).
/// For xpub: 78-byte payload = 4-byte version + 1 depth + 4 fp + 4 child + 32 chain + 33 key
pub fn base58checkDecode(allocator: Allocator, encoded: []const u8) (Base58Error || Allocator.Error)![]u8 {
    // Convert from base58 to big integer (as bytes)
    var result = try allocator.alloc(u8, encoded.len);
    defer allocator.free(result);
    @memset(result, 0);
    var result_len: usize = 1;

    for (encoded) |ch| {
        const val: u8 = for (base58_alphabet, 0..) |a, idx| {
            if (a == ch) break @intCast(idx);
        } else return error.InvalidChar;

        var carry: u16 = val;
        var j: usize = 0;
        while (j < result_len) : (j += 1) {
            carry += @as(u16, result[j]) * 58;
            result[j] = @intCast(carry & 0xff);
            carry >>= 8;
        }
        while (carry > 0) {
            if (result_len >= result.len) {
                // Grow
                const new = try allocator.alloc(u8, result.len * 2);
                @memset(new, 0);
                @memcpy(new[0..result_len], result[0..result_len]);
                allocator.free(result);
                result = new;
            }
            result[result_len] = @intCast(carry & 0xff);
            result_len += 1;
            carry >>= 8;
        }
    }

    // Count leading '1's (= leading zero bytes)
    var leading_zeros: usize = 0;
    for (encoded) |ch| {
        if (ch == '1') {
            leading_zeros += 1;
        } else break;
    }

    // Build output: leading zeros + reversed result
    const total = leading_zeros + result_len;
    if (total < 4) return error.TooShort;
    const decoded = try allocator.alloc(u8, total);
    errdefer allocator.free(decoded);
    @memset(decoded[0..leading_zeros], 0);
    var k: usize = 0;
    while (k < result_len) : (k += 1) {
        decoded[leading_zeros + k] = result[result_len - 1 - k];
    }

    // Verify checksum: last 4 bytes = first 4 of doubleSha256(payload)
    const payload = decoded[0 .. total - 4];
    const checksum = decoded[total - 4 .. total];
    const hash = doubleSha256(payload);
    if (!std.mem.eql(u8, checksum, hash[0..4])) {
        allocator.free(decoded);
        return error.InvalidChecksum;
    }

    // Return just the payload (without checksum), caller frees
    const out = try allocator.alloc(u8, payload.len);
    @memcpy(out, payload);
    allocator.free(decoded);
    return out;
}

/// Extract the 33-byte compressed public key from an xpub string.
/// xpub payload layout: 4 version + 1 depth + 4 fingerprint + 4 child_num + 32 chain_code + 33 pubkey
pub fn pubkeyFromXpub(allocator: Allocator, xpub: []const u8) !struct { pubkey: [33]u8, fingerprint: [4]u8 } {
    const payload = try base58checkDecode(allocator, xpub);
    defer allocator.free(payload);
    if (payload.len < 78) return error.TooShort;
    var pubkey: [33]u8 = undefined;
    @memcpy(&pubkey, payload[45..78]);
    var fp: [4]u8 = undefined;
    @memcpy(&fp, payload[5..9]);
    return .{ .pubkey = pubkey, .fingerprint = fp };
}

// ── Tests ────────────────────────────────────────────────────────

test "doubleSha256" {
    // SHA256d("") is well-known
    const result = doubleSha256("");
    const expected = "5df6e0e2761359d30a8275058e299fcc0381534545f55cf43e41983f5d4c9456";
    var hex: [64]u8 = undefined;
    for (result, 0..) |b, idx| {
        _ = std.fmt.bufPrint(hex[idx * 2 ..][0..2], "{x:0>2}", .{b}) catch unreachable;
    }
    try std.testing.expectEqualStrings(expected, &hex);
}

test "writeCompactSize" {
    var buf: [9]u8 = undefined;

    var pos: usize = 0;
    writeCompactSize(&buf, &pos, 0);
    try std.testing.expectEqual(@as(usize, 1), pos);
    try std.testing.expectEqual(@as(u8, 0), buf[0]);

    pos = 0;
    writeCompactSize(&buf, &pos, 252);
    try std.testing.expectEqual(@as(usize, 1), pos);
    try std.testing.expectEqual(@as(u8, 252), buf[0]);

    pos = 0;
    writeCompactSize(&buf, &pos, 253);
    try std.testing.expectEqual(@as(usize, 3), pos);
    try std.testing.expectEqual(@as(u8, 0xfd), buf[0]);
}

test "serializeTx minimal" {
    const tx = Transaction{
        .version = 0,
        .inputs = &[_]TxInput{.{
            .prevout = .{ .txid = [_]u8{0} ** 32, .vout = 0xffffffff },
            .script_sig = &[_]u8{},
            .sequence = 0,
        }},
        .outputs = &[_]TxOutput{.{
            .value = 0,
            .script_pubkey = &[_]u8{0x6a}, // OP_RETURN
        }},
        .locktime = 0,
    };
    const raw = try serializeTx(std.testing.allocator, tx);
    defer std.testing.allocator.free(raw);
    // version(4) + varint(1) + prevout(36) + varint(1) + scriptsig(0) + seq(4)
    // + varint(1) + value(8) + varint(1) + script(1) + locktime(4) = 61
    try std.testing.expectEqual(@as(usize, 61), raw.len);
    // version = 0 LE
    try std.testing.expectEqual(@as(u8, 0), raw[0]);
}

test "bech32Decode bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4" {
    // This is a well-known P2WPKH testnet/mainnet test vector
    const result = try bech32Decode("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4");
    try std.testing.expectEqual(@as(u8, 0), result.witness_version);
    // The 20-byte witness program for this address
    const expected_hex = "751e76e8199196d454941c45d1b3a323f1433bd6";
    var hex: [40]u8 = undefined;
    for (result.witness_program, 0..) |b, idx| {
        _ = std.fmt.bufPrint(hex[idx * 2 ..][0..2], "{x:0>2}", .{b}) catch unreachable;
    }
    try std.testing.expectEqualStrings(expected_hex, &hex);
}

test "scriptPubkeyFromAddress" {
    const spk = try scriptPubkeyFromAddress("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4");
    try std.testing.expectEqual(@as(u8, 0x00), spk[0]); // OP_0
    try std.testing.expectEqual(@as(u8, 0x14), spk[1]); // PUSH 20
    try std.testing.expectEqual(@as(usize, 22), spk.len);
}
