const std = @import("std");
const hwi = @import("hwi");
const bitcoin = @import("bitcoin.zig");
const bip322 = @import("bip322.zig");

const Allocator = std.mem.Allocator;
const ColdCardDevice = hwi.cc.ColdCardDevice;
const DeviceError = hwi.hwi.DeviceError;

pub const SignError = error{
    NoColdcardFound,
    UserRefused,
    Timeout,
    AddressNotP2wpkh,
    InvalidPath,
    SigningFailed,
    ConnectionFailed,
    XpubDecodeFailed,
    OutOfMemory,
    PsbtError,
};

const HARDENED: u32 = 0x80000000;

/// Parse a BIP32 path string like "m/84'/0'/0'/0/0" into u32 array.
/// Returns the number of path components.
pub fn parseBip32Path(path_str: []const u8, out: *[12]u32) SignError!usize {
    var str = path_str;
    // Skip leading "m/"
    if (str.len >= 2 and str[0] == 'm' and str[1] == '/') {
        str = str[2..];
    } else if (str.len >= 1 and str[0] == 'm') {
        str = str[1..];
    }

    var count: usize = 0;
    var iter = std.mem.splitScalar(u8, str, '/');
    while (iter.next()) |component| {
        if (component.len == 0) continue;
        if (count >= 12) return error.InvalidPath;

        var num_str = component;
        var hardened = false;
        if (num_str.len > 0 and (num_str[num_str.len - 1] == '\'' or num_str[num_str.len - 1] == 'h')) {
            hardened = true;
            num_str = num_str[0 .. num_str.len - 1];
        }

        const val = std.fmt.parseInt(u32, num_str, 10) catch return error.InvalidPath;
        out[count] = if (hardened) val | HARDENED else val;
        count += 1;
    }

    if (count == 0) return error.InvalidPath;
    return count;
}

/// Default P2WPKH derivation path: m/84'/0'/0'/0/0
pub fn defaultP2wpkhPath() struct { path: [5]u32, len: usize } {
    return .{
        .path = .{
            84 | HARDENED,
            0 | HARDENED,
            0 | HARDENED,
            0,
            0,
        },
        .len = 5,
    };
}

/// Complete BIP-322 signing flow using a connected Coldcard.
///
/// 1. Detect and connect to Coldcard
/// 2. Get master fingerprint + xpub to extract pubkey
/// 3. Compute BIP-322 message hash
/// 4. Build to_spend and to_sign transactions
/// 5. Wrap as PSBTv0
/// 6. Sign via Coldcard
/// 7. Extract witness and base64-encode
pub fn signBip322(
    allocator: Allocator,
    message: []const u8,
    address: []const u8,
    path: []const u32,
) SignError![]u8 {
    // Initialize Coldcard device
    var device = ColdCardDevice.init();
    defer device.deinit();

    const detected = device.hwDevice().detect() catch return error.ConnectionFailed;
    if (!detected) return error.NoColdcardFound;

    // Establish encrypted session
    device.encryptSession(allocator) catch return error.ConnectionFailed;

    const hw = device.hwDevice();

    // Get master fingerprint
    const master_fp = hw.getMasterFingerprint(allocator) catch return error.ConnectionFailed;

    // Get xpub at the signing path and extract pubkey
    const xpub_str = hw.getXpub(allocator, path) catch return error.ConnectionFailed;
    defer allocator.free(xpub_str);

    const xpub_info = bitcoin.pubkeyFromXpub(allocator, xpub_str) catch return error.XpubDecodeFailed;
    const pubkey = xpub_info.pubkey;

    // Decode address to scriptPubKey (P2WPKH only for now)
    const spk = bitcoin.scriptPubkeyFromAddress(address) catch return error.AddressNotP2wpkh;

    // Compute BIP-322 message hash
    const msg_hash = bip322.messageHash(message);

    // Build the virtual transactions
    const to_spend = bip322.buildToSpend(msg_hash, &spk);
    const to_spend_txid = bitcoin.txid(allocator, to_spend) catch return error.OutOfMemory;
    const to_sign = bip322.buildToSign(to_spend_txid);

    // Build PSBTv0
    const psbt = bip322.buildPsbtV0(
        allocator,
        to_sign,
        to_spend.outputs[0],
        path,
        master_fp,
        pubkey,
    ) catch return error.OutOfMemory;
    defer allocator.free(psbt);

    // Sign the PSBT via Coldcard
    const signed_psbt = hw.signTx(allocator, psbt) catch |err| {
        return switch (err) {
            DeviceError.UserRefused => error.UserRefused,
            DeviceError.Timeout => error.Timeout,
            else => error.SigningFailed,
        };
    };
    defer allocator.free(signed_psbt);

    // Extract the witness from the signed PSBT
    const witness = bip322.extractWitness(signed_psbt) catch return error.PsbtError;

    // Base64-encode the witness as the BIP-322 "simple" signature
    return bip322.encodeSignature(allocator, witness) catch return error.OutOfMemory;
}

// ── Tests ────────────────────────────────────────────────────────

test "parseBip32Path standard" {
    var buf: [12]u32 = undefined;
    const len = try parseBip32Path("m/84'/0'/0'/0/0", &buf);
    try std.testing.expectEqual(@as(usize, 5), len);
    try std.testing.expectEqual(84 | HARDENED, buf[0]);
    try std.testing.expectEqual(0 | HARDENED, buf[1]);
    try std.testing.expectEqual(0 | HARDENED, buf[2]);
    try std.testing.expectEqual(@as(u32, 0), buf[3]);
    try std.testing.expectEqual(@as(u32, 0), buf[4]);
}

test "parseBip32Path with h notation" {
    var buf: [12]u32 = undefined;
    const len = try parseBip32Path("m/49h/0h/0h/0/1", &buf);
    try std.testing.expectEqual(@as(usize, 5), len);
    try std.testing.expectEqual(49 | HARDENED, buf[0]);
    try std.testing.expectEqual(@as(u32, 1), buf[4]);
}

test "parseBip32Path no m prefix" {
    var buf: [12]u32 = undefined;
    const len = try parseBip32Path("84'/0'/0'/0/0", &buf);
    try std.testing.expectEqual(@as(usize, 5), len);
    try std.testing.expectEqual(84 | HARDENED, buf[0]);
}

test "defaultP2wpkhPath" {
    const p = defaultP2wpkhPath();
    try std.testing.expectEqual(@as(usize, 5), p.len);
    try std.testing.expectEqual(84 | HARDENED, p.path[0]);
}
