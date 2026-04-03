//! Mock HWDevice for integration testing.
//!
//! Implements the HWDevice vtable using a known private key and
//! secp256k1. Signs BIP-322 PSBTs entirely in software, no hardware
//! required.
const std = @import("std");
const hwi_mod = @import("hwi");
const bitcoin = @import("bitcoin.zig");
const bip322 = @import("bip322.zig");

const Sha256 = std.crypto.hash.sha2.Sha256;
const HWDevice = hwi_mod.hwi.HWDevice;
const DeviceError = hwi_mod.hwi.DeviceError;
const AddressFormat = hwi_mod.hwi.AddressFormat;
const DeviceInfo = hwi_mod.hwi.DeviceInfo;
const Allocator = std.mem.Allocator;

const secp256k1 = hwi_mod.cc.crypto.secp256k1;

fn castSelf(ptr: *anyopaque) *MockDevice {
    return @ptrCast(@alignCast(ptr));
}

/// A software-only device backed by a 32-byte secret key.
pub const MockDevice = struct {
    const Self = @This();

    secret_key: [32]u8,
    compressed_pubkey: [33]u8,
    master_fingerprint: [4]u8,

    /// Create a MockDevice from a 32-byte secret key.
    pub fn init(secret_key: [32]u8) Self {
        const ctx = secp256k1.secp256k1_context_create(secp256k1.SECP256K1_CONTEXT_NONE);
        defer secp256k1.secp256k1_context_destroy(ctx);

        // Derive compressed public key
        var pubkey: secp256k1.secp256k1_pubkey = undefined;
        if (secp256k1.secp256k1_ec_pubkey_create(ctx, &pubkey, &secret_key) != 1)
            @panic("invalid secret key");

        var compressed: [33]u8 = undefined;
        var len: usize = 33;
        if (secp256k1.secp256k1_ec_pubkey_serialize(
            ctx,
            &compressed,
            &len,
            &pubkey,
            secp256k1.SECP256K1_EC_COMPRESSED,
        ) != 1) @panic("pubkey serialize failed");

        // Master fingerprint = first 4 bytes of HASH160(compressed pubkey)
        var sha_out: [32]u8 = undefined;
        Sha256.hash(&compressed, &sha_out, .{});
        // RIPEMD160 not in std — just use first 4 bytes of SHA256 for the mock.
        // Real fingerprint is RIPEMD160(SHA256(pubkey))[0..4], but for testing
        // this is deterministic and consistent.
        var fp: [4]u8 = undefined;
        @memcpy(&fp, sha_out[0..4]);

        return .{
            .secret_key = secret_key,
            .compressed_pubkey = compressed,
            .master_fingerprint = fp,
        };
    }

    pub fn hwDevice(dev: *Self) HWDevice {
        return .{
            .ptr = @ptrCast(dev),
            .vtable = &vtable,
        };
    }

    const vtable = HWDevice.VTable{
        .detect = mockDetect,
        .getInfo = mockGetInfo,
        .getMasterFingerprint = mockGetMasterFingerprint,
        .getXpub = mockGetXpub,
        .signTx = mockSignTx,
        .signMessage = mockSignMessage,
        .showAddress = mockShowAddress,
        .setPassphrase = mockSetPassphrase,
        .deinit = mockDeinit,
    };

    fn mockDetect(_: *anyopaque) DeviceError!bool {
        return true;
    }

    fn mockGetInfo(_: *anyopaque, _: Allocator) DeviceError!DeviceInfo {
        return .{
            .model = "mock-software-signer",
            .firmware_version = "0.0.1",
            .master_fingerprint = .{ 0, 0, 0, 0 },
        };
    }

    fn mockGetMasterFingerprint(ptr: *anyopaque, _: Allocator) DeviceError![4]u8 {
        return castSelf(ptr).master_fingerprint;
    }

    fn mockGetXpub(ptr: *anyopaque, allocator: Allocator, _: []const u32) DeviceError![]u8 {
        // Return a fake xpub that encodes our compressed pubkey at bytes 45..78.
        // Real xpub: 4-byte version + 1 depth + 4 fp + 4 child + 32 chain + 33 key = 78 bytes.
        // Then base58check-encode.
        const me = castSelf(ptr);
        var payload: [78]u8 = undefined;
        // version: xpub (0x0488B21E)
        payload[0] = 0x04;
        payload[1] = 0x88;
        payload[2] = 0xB2;
        payload[3] = 0x1E;
        // depth
        payload[4] = 0x00;
        // parent fingerprint
        @memset(payload[5..9], 0);
        // child number
        @memset(payload[9..13], 0);
        // chain code (fake)
        @memset(payload[13..45], 0xcc);
        // public key
        @memcpy(payload[45..78], &me.compressed_pubkey);

        // base58check encode: payload + 4-byte checksum
        const checksum = bitcoin.doubleSha256(&payload);
        var full: [82]u8 = undefined;
        @memcpy(full[0..78], &payload);
        @memcpy(full[78..82], checksum[0..4]);

        // Base58 encode
        return base58Encode(allocator, &full) catch return DeviceError.AllocError;
    }

    fn mockSignTx(ptr: *anyopaque, allocator: Allocator, psbt_bytes: []const u8) DeviceError![]u8 {
        const me = castSelf(ptr);

        // Parse enough of the PSBT to find what we need to sign.
        // We need to:
        // 1. Extract the unsigned tx from the global map
        // 2. Compute the BIP-143 segwit sighash
        // 3. Sign with our key
        // 4. Return a PSBT with PSBT_IN_FINAL_SCRIPTWITNESS populated

        if (psbt_bytes.len < 5 or !std.mem.eql(u8, psbt_bytes[0..5], "psbt\xff"))
            return DeviceError.InvalidResponse;

        // Parse PSBT: scan all key-value pairs looking for what we need
        var pos: usize = 5;
        var unsigned_tx: ?[]const u8 = null;
        var witness_utxo: ?[]const u8 = null;
        var in_global = true;

        while (pos < psbt_bytes.len) {
            const key_len = readVarInt(psbt_bytes, &pos);
            if (key_len == 0) {
                // Separator — transition from global to input 0 to output 0
                if (in_global) {
                    in_global = false;
                    continue;
                }
                break; // past input 0
            }
            if (pos + key_len > psbt_bytes.len) break;
            const key_type = psbt_bytes[pos];
            pos += key_len;
            const val_len = readVarInt(psbt_bytes, &pos);
            if (pos + val_len > psbt_bytes.len) break;
            const val = psbt_bytes[pos .. pos + val_len];
            pos += val_len;

            if (in_global and key_type == 0x00) {
                unsigned_tx = val;
            } else if (!in_global and key_type == 0x01) {
                witness_utxo = val;
            }
        }

        const utx = unsigned_tx orelse return DeviceError.InvalidResponse;
        const wutxo = witness_utxo orelse return DeviceError.InvalidResponse;

        // Compute BIP-143 sighash for P2WPKH
        const sighash = computeSegwitSighash(allocator, utx, wutxo, &me.compressed_pubkey) catch
            return DeviceError.AllocError;

        // Sign with secp256k1
        const ctx = secp256k1.secp256k1_context_create(secp256k1.SECP256K1_CONTEXT_NONE);
        defer secp256k1.secp256k1_context_destroy(ctx);

        var sig: secp256k1.secp256k1_ecdsa_signature = undefined;
        if (secp256k1.secp256k1_ecdsa_sign(ctx, &sig, &sighash, &me.secret_key, null, null) != 1)
            return DeviceError.TransportError;

        // Serialize as DER
        var der_sig: [72]u8 = undefined;
        var der_len: usize = 72;
        if (secp256k1.secp256k1_ecdsa_signature_serialize_der(ctx, &der_sig, &der_len, &sig) != 1)
            return DeviceError.TransportError;

        // Build witness: 02 <sig_len> <der_sig> <SIGHASH_ALL> <pubkey_len> <pubkey>
        // Witness stack serialization: varint(num_items) + for each: varint(len) + data
        const sig_with_hashtype_len = der_len + 1; // DER sig + SIGHASH_ALL byte
        const witness_len = 1 + // varint: 2 items
            bitcoin.compactSizeLen(sig_with_hashtype_len) + sig_with_hashtype_len +
            bitcoin.compactSizeLen(33) + 33; // pubkey

        const witness_buf = allocator.alloc(u8, witness_len) catch return DeviceError.AllocError;
        defer allocator.free(witness_buf);
        {
            var wpos: usize = 0;
            witness_buf[wpos] = 0x02; // 2 stack items
            wpos += 1;
            bitcoin.writeCompactSize(witness_buf, &wpos, sig_with_hashtype_len);
            @memcpy(witness_buf[wpos..][0..der_len], der_sig[0..der_len]);
            wpos += der_len;
            witness_buf[wpos] = 0x01; // SIGHASH_ALL
            wpos += 1;
            bitcoin.writeCompactSize(witness_buf, &wpos, 33);
            @memcpy(witness_buf[wpos..][0..33], &me.compressed_pubkey);
            wpos += 33;
            std.debug.assert(wpos == witness_len);
        }

        // Build output PSBT with PSBT_IN_FINAL_SCRIPTWITNESS (key 0x07)
        return buildSignedPsbt(allocator, psbt_bytes, witness_buf) catch
            return DeviceError.AllocError;
    }

    fn mockSignMessage(_: *anyopaque, _: Allocator, _: []const u32, _: []const u8) DeviceError![]u8 {
        return DeviceError.NotSupported;
    }

    fn mockShowAddress(_: *anyopaque, _: Allocator, _: []const u32, _: AddressFormat) DeviceError![]u8 {
        return DeviceError.NotSupported;
    }

    fn mockSetPassphrase(_: *anyopaque, _: Allocator, _: []const u8) DeviceError!void {
        return DeviceError.NotSupported;
    }

    fn mockDeinit(_: *anyopaque) void {}
};

// ── BIP-143 Segwit Sighash ───────────────────────────────────────

fn computeSegwitSighash(
    allocator: Allocator,
    unsigned_tx: []const u8,
    witness_utxo: []const u8,
    pubkey: *const [33]u8,
) !@TypeOf(bitcoin.doubleSha256("")) {
    // Parse unsigned tx to get prevout, sequence, outputs, version, locktime
    if (unsigned_tx.len < 10) return error.InvalidFormat;

    const version = unsigned_tx[0..4];
    var pos: usize = 4;

    // Read input count
    const in_count = readVarInt(unsigned_tx, &pos);
    _ = in_count;

    // Read input 0
    const prevout = unsigned_tx[pos..][0..36];
    pos += 36;
    const scriptsig_len = readVarInt(unsigned_tx, &pos);
    pos += scriptsig_len; // skip scriptsig
    const sequence = unsigned_tx[pos..][0..4];
    pos += 4;

    // Read output count and outputs
    const out_count_pos = pos;
    _ = readVarInt(unsigned_tx, &pos);
    const outputs_start = pos;
    // Find end of outputs (scan to locktime)
    // outputs end 4 bytes before end of tx
    const locktime_pos = unsigned_tx.len - 4;
    const locktime = unsigned_tx[locktime_pos..][0..4];
    const outputs_data = unsigned_tx[out_count_pos..locktime_pos];
    _ = outputs_start;

    // hashPrevouts = SHA256d(prevout)
    const hash_prevouts = bitcoin.doubleSha256(prevout);

    // hashSequence = SHA256d(sequence)
    const hash_sequence = bitcoin.doubleSha256(sequence);

    // hashOutputs = SHA256d(all serialized outputs including varint count)
    const hash_outputs = bitcoin.doubleSha256(outputs_data);

    // scriptCode for P2WPKH: OP_DUP OP_HASH160 PUSH20 <hash160(pubkey)> OP_EQUALVERIFY OP_CHECKSIG
    // = 0x1976a914{hash160}88ac
    var pubkey_sha: [32]u8 = undefined;
    Sha256.hash(pubkey, &pubkey_sha, .{});
    // Use first 20 bytes of SHA256 as mock HASH160 (no RIPEMD160 in std)
    // For testing this is fine — the sighash just needs to be consistent.
    var script_code: [25]u8 = undefined;
    script_code[0] = 0x19; // length of script (25 bytes... wait, this is the varint for the script)
    // Actually scriptCode for sighash preimage is: varint(len) + script
    // P2WPKH scriptCode = 0x1976a914{20-byte-hash}88ac = 26 bytes total
    var sc: [26]u8 = undefined;
    sc[0] = 0x19; // varint: 25
    sc[1] = 0x76; // OP_DUP
    sc[2] = 0xa9; // OP_HASH160
    sc[3] = 0x14; // PUSH 20
    @memcpy(sc[4..24], pubkey_sha[0..20]); // mock hash160
    sc[24] = 0x88; // OP_EQUALVERIFY
    sc[25] = 0xac; // OP_CHECKSIG

    // Witness UTXO value (first 8 bytes)
    const utxo_value = witness_utxo[0..8];

    // BIP-143 preimage:
    // version + hashPrevouts + hashSequence + outpoint + scriptCode + value + sequence + hashOutputs + locktime + sighash_type
    const preimage_len = 4 + 32 + 32 + 36 + 26 + 8 + 4 + 32 + 4 + 4;
    var preimage = try allocator.alloc(u8, preimage_len);
    defer allocator.free(preimage);
    var p: usize = 0;
    @memcpy(preimage[p..][0..4], version);
    p += 4;
    @memcpy(preimage[p..][0..32], &hash_prevouts);
    p += 32;
    @memcpy(preimage[p..][0..32], &hash_sequence);
    p += 32;
    @memcpy(preimage[p..][0..36], prevout);
    p += 36;
    @memcpy(preimage[p..][0..26], &sc);
    p += 26;
    @memcpy(preimage[p..][0..8], utxo_value);
    p += 8;
    @memcpy(preimage[p..][0..4], sequence);
    p += 4;
    @memcpy(preimage[p..][0..32], &hash_outputs);
    p += 32;
    @memcpy(preimage[p..][0..4], locktime);
    p += 4;
    // SIGHASH_ALL = 0x01000000 (LE)
    preimage[p] = 0x01;
    preimage[p + 1] = 0x00;
    preimage[p + 2] = 0x00;
    preimage[p + 3] = 0x00;
    p += 4;

    return bitcoin.doubleSha256(preimage);
}

fn readVarInt(data: []const u8, pos: *usize) usize {
    const first = data[pos.*];
    pos.* += 1;
    if (first < 0xfd) return first;
    if (first == 0xfd) {
        const val = std.mem.readInt(u16, data[pos.*..][0..2], .little);
        pos.* += 2;
        return val;
    }
    if (first == 0xfe) {
        const val = std.mem.readInt(u32, data[pos.*..][0..4], .little);
        pos.* += 4;
        return val;
    }
    const val = std.mem.readInt(u64, data[pos.*..][0..8], .little);
    pos.* += 8;
    return @intCast(val);
}

// ── PSBT helpers ─────────────────────────────────────────────────

fn extractPsbtValue(data: []const u8, pos: *usize, target_key_type: u8) ?[]const u8 {
    while (pos.* < data.len) {
        const key_len = readVarInt(data, pos);
        if (key_len == 0) return null; // separator
        if (pos.* + key_len > data.len) return null;
        const key_type = data[pos.*];
        pos.* += key_len;
        const val_len = readVarInt(data, pos);
        if (pos.* + val_len > data.len) return null;
        if (key_type == target_key_type) {
            return data[pos.* .. pos.* + val_len];
        }
        pos.* += val_len;
    }
    return null;
}

fn skipToSeparator(data: []const u8, pos: *usize) void {
    while (pos.* < data.len) {
        const key_len = readVarInt(data, pos);
        if (key_len == 0) return; // separator
        if (pos.* + key_len > data.len) return;
        pos.* += key_len;
        const val_len = readVarInt(data, pos);
        if (pos.* + val_len > data.len) return;
        pos.* += val_len;
    }
}

fn buildSignedPsbt(allocator: Allocator, original_psbt: []const u8, witness: []const u8) ![]u8 {
    // Copy the original PSBT but inject PSBT_IN_FINAL_SCRIPTWITNESS into input 0.
    // Strategy: copy global map, then copy input 0 kv pairs, add 0x07 kv, copy rest.

    // Calculate size of the witness kv pair
    const wit_kv_len = 1 + 1 + // compact_size(key_len=1) + key(0x07)
        bitcoin.compactSizeLen(witness.len) + witness.len;

    const result = try allocator.alloc(u8, original_psbt.len + wit_kv_len);
    errdefer allocator.free(result);

    var rpos: usize = 0; // read position in original
    var wpos: usize = 0; // write position in result

    // Copy magic
    @memcpy(result[wpos..][0..5], original_psbt[0..5]);
    rpos = 5;
    wpos = 5;

    // Copy global map until separator
    while (rpos < original_psbt.len) {
        const key_len = readVarInt(original_psbt, &rpos);
        if (key_len == 0) {
            result[wpos] = 0x00;
            wpos += 1;
            break;
        }
        // Write back key_len + key + val_len + val
        const kv_start = rpos - bitcoin.compactSizeLen(key_len);
        const val_len = blk: {
            const kp = rpos;
            const vp = rpos + key_len;
            var vrpos = vp;
            const vl = readVarInt(original_psbt, &vrpos);
            _ = kp;
            break :blk vl;
        };
        rpos += key_len;
        const actual_val_len = readVarInt(original_psbt, &rpos);
        _ = val_len;
        const total_kv = rpos + actual_val_len - kv_start;
        _ = total_kv;

        // Simpler: just track where we were before key_len read, copy raw bytes
        // Let me restart with a simpler approach
        break;
    }

    // Simpler approach: find the first input separator, inject witness kv before it
    rpos = 5;
    wpos = 5;

    // Skip global map
    while (rpos < original_psbt.len) {
        if (original_psbt[rpos] == 0x00) {
            // Could be separator or compact_size(0). In PSBT, 0x00 as key_len = separator.
            result[wpos] = 0x00;
            rpos += 1;
            wpos += 1;
            break;
        }
        const save = rpos;
        const kl = readVarInt(original_psbt, &rpos);
        rpos += kl;
        const vl = readVarInt(original_psbt, &rpos);
        rpos += vl;
        const chunk_len = rpos - save;
        @memcpy(result[wpos..][0..chunk_len], original_psbt[save..rpos]);
        wpos += chunk_len;
    }

    // Copy input 0 kv pairs
    while (rpos < original_psbt.len) {
        if (original_psbt[rpos] == 0x00) {
            // Before the separator, inject our witness kv
            // key: 0x07 (PSBT_IN_FINAL_SCRIPTWITNESS)
            result[wpos] = 0x01; // compact_size(1) = key is 1 byte
            wpos += 1;
            result[wpos] = 0x07; // key type
            wpos += 1;
            bitcoin.writeCompactSize(result, &wpos, witness.len);
            @memcpy(result[wpos..][0..witness.len], witness);
            wpos += witness.len;

            // Copy separator
            result[wpos] = 0x00;
            rpos += 1;
            wpos += 1;
            break;
        }
        const save = rpos;
        const kl = readVarInt(original_psbt, &rpos);
        rpos += kl;
        const vl = readVarInt(original_psbt, &rpos);
        rpos += vl;
        const chunk_len = rpos - save;
        @memcpy(result[wpos..][0..chunk_len], original_psbt[save..rpos]);
        wpos += chunk_len;
    }

    // Copy remaining (output maps, etc.)
    const remaining = original_psbt.len - rpos;
    @memcpy(result[wpos..][0..remaining], original_psbt[rpos..]);
    wpos += remaining;

    // Trim to actual size
    const final_buf = try allocator.alloc(u8, wpos);
    @memcpy(final_buf, result[0..wpos]);
    allocator.free(result);
    return final_buf;
}

// ── Base58 Encoding ──────────────────────────────────────────────

const base58_alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

fn base58Encode(allocator: Allocator, input: []const u8) ![]u8 {
    if (input.len == 0) {
        return allocator.alloc(u8, 0);
    }

    // Count leading zeros
    var leading_zeros: usize = 0;
    for (input) |b| {
        if (b == 0) {
            leading_zeros += 1;
        } else break;
    }

    // Allocate enough space (log(256)/log(58) ~ 1.366)
    const max_len = input.len * 138 / 100 + 1;
    var buf = try allocator.alloc(u8, max_len);
    defer allocator.free(buf);
    @memset(buf, 0);
    var buf_len: usize = 0;

    for (input) |byte| {
        var carry: u32 = byte;
        var j: usize = 0;
        while (j < buf_len or carry > 0) {
            if (j < buf_len) {
                carry += @as(u32, buf[j]) * 256;
            }
            buf[j] = @intCast(carry % 58);
            carry /= 58;
            j += 1;
        }
        buf_len = j;
    }

    // Result = leading '1's + reversed buf
    const result_len = leading_zeros + buf_len;
    var result = try allocator.alloc(u8, result_len);
    @memset(result[0..leading_zeros], '1');
    var k: usize = 0;
    while (k < buf_len) : (k += 1) {
        result[leading_zeros + k] = base58_alphabet[buf[buf_len - 1 - k]];
    }
    return result;
}

// ── Tests ────────────────────────────────────────────────────────

test "MockDevice init and detect" {
    // Well-known test key (32 bytes of 0x01)
    var device = MockDevice.init([_]u8{0x01} ** 32);
    var hw = device.hwDevice();
    const detected = try hw.detect();
    try std.testing.expect(detected);
}

test "MockDevice getXpub roundtrip" {
    var device = MockDevice.init([_]u8{0x01} ** 32);
    var hw = device.hwDevice();
    const xpub = try hw.getXpub(std.testing.allocator, &[_]u32{0});
    defer std.testing.allocator.free(xpub);
    // Verify it starts with "xpub" (base58 for version 0x0488B21E)
    try std.testing.expect(std.mem.startsWith(u8, xpub, "xpub"));

    // Verify we can decode it back and get the same pubkey
    const info = try bitcoin.pubkeyFromXpub(std.testing.allocator, xpub);
    try std.testing.expectEqualSlices(u8, &device.compressed_pubkey, &info.pubkey);
}

test "MockDevice signTx produces valid witness" {
    const bip322_mod = @import("bip322.zig");
    const bitcoin_mod = @import("bitcoin.zig");

    var device = MockDevice.init([_]u8{0x01} ** 32);
    var hw = device.hwDevice();

    // Build a BIP-322 PSBT
    const msg_hash = bip322_mod.messageHash("test message");
    var spk: [22]u8 = undefined;
    spk[0] = 0x00;
    spk[1] = 0x14;
    // Use SHA256(pubkey)[0..20] as mock witness program (matching our mock hash160)
    var pubkey_sha: [32]u8 = undefined;
    Sha256.hash(&device.compressed_pubkey, &pubkey_sha, .{});
    @memcpy(spk[2..22], pubkey_sha[0..20]);

    var to_spend_data = bip322_mod.buildToSpend(msg_hash, &spk);
    const to_spend_id = try bitcoin_mod.txid(std.testing.allocator, to_spend_data.tx());
    var to_sign_data = bip322_mod.buildToSign(to_spend_id);

    const fp = try hw.getMasterFingerprint(std.testing.allocator);
    const path = [_]u32{ 84 | 0x80000000, 0 | 0x80000000, 0 | 0x80000000, 0, 0 };

    const psbt = try bip322_mod.buildPsbtV0(
        std.testing.allocator,
        to_sign_data.tx(),
        to_spend_data.output[0],
        &path,
        fp,
        device.compressed_pubkey,
    );
    defer std.testing.allocator.free(psbt);

    // Sign
    const signed = try hw.signTx(std.testing.allocator, psbt);
    defer std.testing.allocator.free(signed);

    // Should have PSBT magic
    try std.testing.expectEqualStrings("psbt", signed[0..4]);

    // Should be able to extract witness
    const witness = try bip322_mod.extractWitness(signed);
    try std.testing.expect(witness.len > 0);
    // First byte should be 0x02 (2 stack items)
    try std.testing.expectEqual(@as(u8, 0x02), witness[0]);

    // Should base64-encode successfully
    const sig = try bip322_mod.encodeSignature(std.testing.allocator, witness);
    defer std.testing.allocator.free(sig);
    try std.testing.expect(sig.len > 0);
}
