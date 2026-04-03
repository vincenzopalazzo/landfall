const std = @import("std");
const lexe = @import("lexe");
const ocean = @import("ocean.zig");
const signer = @import("signer.zig");

const Allocator = std.mem.Allocator;

// ── CLI Types ────────────────────────────────────────────────────

const Command = union(enum) {
    sign: SignArgs,
    info,
    configure: ConfigureArgs,
    invoice: InvoiceArgs,
    pay: PayArgs,
    payment: PaymentArgs,
    health,
    help,
};

const SignArgs = struct {
    message: []const u8,
    address: ?[]const u8,
    path: ?[]const u8,
};

const ConfigureArgs = struct {
    /// Bitcoin address registered with OCEAN (for signing context).
    address: ?[]const u8,
    /// The OCEAN configuration message to sign (provided by the user).
    message: ?[]const u8,
    /// BOLT12 offer override — if null, fetched from Lexe node.
    offer: ?[]const u8,
};

const InvoiceArgs = struct {
    amount: []const u8,
    description: ?[]const u8,
};

const PayArgs = struct {
    bolt11: []const u8,
};

const PaymentArgs = struct {
    index: []const u8,
};

const CliArgs = struct {
    url: []const u8,
    credentials: ?[]const u8,
    command: Command,
    json: bool,
};

const ParseError = error{
    MissingFlagValue,
    UnknownFlag,
    NoCommand,
    UnknownCommand,
    MissingAmount,
    MissingInvoice,
    MissingIndex,
};

// ── Arg Parsing ──────────────────────────────────────────────────

fn parseArgs(args: []const []const u8) ParseError!CliArgs {
    var url: []const u8 = "http://127.0.0.1:5393";
    var credentials: ?[]const u8 = null;
    var json: bool = false;
    var positional: [8][]const u8 = undefined;
    var pos_count: usize = 0;

    // Shared flags
    var cfg_address: ?[]const u8 = null;
    var cfg_message: ?[]const u8 = null;
    var cfg_offer: ?[]const u8 = null;
    var message_to_sign: ?[]const u8 = null;
    var sign_path: ?[]const u8 = null;

    var i: usize = 1; // skip argv[0]
    while (i < args.len) : (i += 1) {
        const arg = args[i];
        if (std.mem.eql(u8, arg, "--url")) {
            i += 1;
            if (i >= args.len) return error.MissingFlagValue;
            url = args[i];
        } else if (std.mem.eql(u8, arg, "--credentials")) {
            i += 1;
            if (i >= args.len) return error.MissingFlagValue;
            credentials = args[i];
        } else if (std.mem.eql(u8, arg, "--json")) {
            json = true;
        } else if (std.mem.eql(u8, arg, "--address")) {
            i += 1;
            if (i >= args.len) return error.MissingFlagValue;
            cfg_address = args[i];
        } else if (std.mem.eql(u8, arg, "--message")) {
            i += 1;
            if (i >= args.len) return error.MissingFlagValue;
            cfg_message = args[i];
        } else if (std.mem.eql(u8, arg, "--message-to-sign")) {
            i += 1;
            if (i >= args.len) return error.MissingFlagValue;
            message_to_sign = args[i];
        } else if (std.mem.eql(u8, arg, "--path")) {
            i += 1;
            if (i >= args.len) return error.MissingFlagValue;
            sign_path = args[i];
        } else if (std.mem.eql(u8, arg, "--offer")) {
            i += 1;
            if (i >= args.len) return error.MissingFlagValue;
            cfg_offer = args[i];
        } else if (std.mem.eql(u8, arg, "--help") or std.mem.eql(u8, arg, "-h")) {
            return .{ .url = url, .credentials = credentials, .command = .help, .json = false };
        } else if (std.mem.startsWith(u8, arg, "--")) {
            return error.UnknownFlag;
        } else {
            if (pos_count < positional.len) {
                positional[pos_count] = arg;
                pos_count += 1;
            }
        }
    }

    // If --message-to-sign is provided, that's the sign command (no subcommand needed)
    if (message_to_sign) |msg| {
        return .{
            .url = url,
            .credentials = credentials,
            .command = .{ .sign = .{
                .message = msg,
                .address = cfg_address,
                .path = sign_path,
            } },
            .json = json,
        };
    }

    if (pos_count == 0) return error.NoCommand;

    const cmd_str = positional[0];
    const rest = positional[1..pos_count];

    const command: Command = if (std.mem.eql(u8, cmd_str, "info"))
        .info
    else if (std.mem.eql(u8, cmd_str, "configure"))
        .{ .configure = .{
            .address = cfg_address,
            .message = cfg_message,
            .offer = cfg_offer,
        } }
    else if (std.mem.eql(u8, cmd_str, "invoice")) blk: {
        if (rest.len == 0) return error.MissingAmount;
        break :blk .{ .invoice = .{
            .amount = rest[0],
            .description = if (rest.len > 1) rest[1] else null,
        } };
    } else if (std.mem.eql(u8, cmd_str, "pay")) blk: {
        if (rest.len == 0) return error.MissingInvoice;
        break :blk .{ .pay = .{ .bolt11 = rest[0] } };
    } else if (std.mem.eql(u8, cmd_str, "payment")) blk: {
        if (rest.len == 0) return error.MissingIndex;
        break :blk .{ .payment = .{ .index = rest[0] } };
    } else if (std.mem.eql(u8, cmd_str, "health"))
        .health
    else if (std.mem.eql(u8, cmd_str, "help"))
        .help
    else
        return error.UnknownCommand;

    return .{
        .url = url,
        .credentials = credentials,
        .command = command,
        .json = json,
    };
}

fn printUsage(w: *std.Io.Writer) !void {
    try w.print(
        \\Usage: oceanln [options] <command> [args]
        \\       oceanln --message-to-sign <msg> [--address <addr>] [--path <path>]
        \\
        \\BIP-322 Signing (via Coldcard):
        \\  --message-to-sign <msg>     Sign a message with the connected Coldcard
        \\  --address <btc_addr>        Bitcoin address (P2WPKH bc1q...) for signing
        \\  --path <bip32_path>         Derivation path (default: m/84'/0'/0'/0/0)
        \\
        \\Wallet Commands:
        \\  configure                   Build OCEAN payout configuration message
        \\  info                        Show Lexe node info (balance, channels, keys)
        \\  invoice <amount> [desc]     Create a BOLT11 invoice (amount in sats)
        \\  pay <bolt11>                Pay a BOLT11 invoice
        \\  payment <index>             Look up a payment by index
        \\  health                      Lexe sidecar health check
        \\  help                        Show this help
        \\
        \\Global options:
        \\  --url <url>                 Sidecar URL (default: http://127.0.0.1:5393)
        \\  --credentials <token>       Bearer credentials for authentication
        \\  --json                      Output as JSON
        \\
        \\Configure options:
        \\  --offer <bolt12>            BOLT12 offer (fetched from Lexe if omitted)
        \\  --message <msg>             OCEAN configuration message (from web UI)
        \\
        \\Examples:
        \\  oceanln --message-to-sign "Configure OCEAN payout to lno1... at block 840000" --address bc1q...
        \\  oceanln info
        \\  oceanln configure --offer lno1... --address bc1q... --message "..."
        \\  oceanln invoice 1000 "donation"
        \\
    , .{});
}

// ── Command Handlers ─────────────────────────────────────────────

fn exitErr(w: *std.Io.Writer, comptime fmt: []const u8, args: anytype) noreturn {
    w.print(fmt, args) catch {};
    w.flush() catch {};
    std.process.exit(1);
}

fn writeJson(allocator: Allocator, w: *std.Io.Writer, value: anytype) !void {
    const json_str = std.json.Stringify.valueAlloc(allocator, value, .{
        .emit_null_optional_fields = false,
        .whitespace = .indent_2,
    }) catch return error.WriteFailed;
    defer allocator.free(json_str);
    try w.writeAll(json_str);
    try w.writeAll("\n");
}

fn cmdSign(allocator: Allocator, args: SignArgs, w: *std.Io.Writer, ew: *std.Io.Writer, json: bool) !void {
    // Parse derivation path
    var path_buf: [12]u32 = undefined;
    var path_len: usize = undefined;
    if (args.path) |p| {
        path_len = signer.parseBip32Path(p, &path_buf) catch
            exitErr(ew, "error: invalid BIP32 path format (expected m/84'/0'/0'/0/0)\n", .{});
    } else {
        const default = signer.defaultP2wpkhPath();
        path_buf[0..5].* = default.path;
        path_len = default.len;
    }

    const address = args.address orelse
        exitErr(ew, "error: --address is required for BIP-322 signing\n", .{});

    try w.print("Signing with Coldcard via BIP-322...\n", .{});
    try w.print("Message: {s}\n", .{args.message});
    try w.print("Address: {s}\n", .{address});
    try w.flush();

    const signature = signer.signBip322(
        allocator,
        args.message,
        address,
        path_buf[0..path_len],
    ) catch |err| {
        switch (err) {
            signer.SignError.NoColdcardFound => exitErr(ew, "error: no Coldcard found. Connect your device via USB.\n", .{}),
            signer.SignError.UserRefused => exitErr(ew, "error: signing refused on device.\n", .{}),
            signer.SignError.Timeout => exitErr(ew, "error: signing timed out. Approve on the Coldcard.\n", .{}),
            signer.SignError.AddressNotP2wpkh => exitErr(ew, "error: only P2WPKH (bc1q...) addresses are supported.\n", .{}),
            signer.SignError.ConnectionFailed => exitErr(ew, "error: failed to communicate with Coldcard.\n", .{}),
            signer.SignError.XpubDecodeFailed => exitErr(ew, "error: failed to decode xpub from Coldcard.\n", .{}),
            signer.SignError.SigningFailed => exitErr(ew, "error: signing failed on device.\n", .{}),
            signer.SignError.PsbtError => exitErr(ew, "error: invalid PSBT response from device.\n", .{}),
            else => exitErr(ew, "error: signing failed\n", .{}),
        }
    };
    defer allocator.free(signature);

    if (json) {
        const output = SignOutput{
            .message = args.message,
            .address = address,
            .signature = signature,
        };
        try writeJson(allocator, w, output);
    } else {
        try w.print("\nBIP-322 Signature:\n{s}\n", .{signature});
        try w.print("\nPaste this signature into the OCEAN web interface.\n", .{});
    }
}

const SignOutput = struct {
    message: []const u8,
    address: []const u8,
    signature: []const u8,
};

fn handleApiError(ew: *std.Io.Writer, resp: anytype) noreturn {
    defer resp.deinit();
    exitErr(ew, "error: API ({d}): {s}\n", .{ resp.value.code, resp.value.msg });
}

fn cmdInfo(allocator: Allocator, client: *lexe.LexeClient, w: *std.Io.Writer, ew: *std.Io.Writer, json: bool) !void {
    const result = client.nodeInfo() catch |err| exitErr(ew, "error: {}\n", .{err});
    switch (result) {
        .ok => |resp| {
            defer resp.deinit();
            const info = resp.value;
            if (json) {
                try writeJson(allocator, w, info);
            } else {
                try w.print("Version:             {s}\n", .{info.version});
                try w.print("Node PK:             {s}\n", .{info.node_pk});
                try w.print("User PK:             {s}\n", .{info.user_pk});
                try w.print("Balance:             {s} sats\n", .{info.balance});
                try w.print("  Lightning:         {s} sats\n", .{info.lightning_balance});
                try w.print("  Sendable:          {s} sats\n", .{info.lightning_sendable_balance});
                try w.print("  On-chain:          {s} sats\n", .{info.onchain_balance});
                try w.print("Channels:            {d} ({d} usable)\n", .{
                    info.num_channels,
                    info.num_usable_channels,
                });
            }
        },
        .err => |resp| handleApiError(ew, resp),
        .not_found => exitErr(ew, "error: unexpected 404\n", .{}),
    }
}

fn cmdConfigure(allocator: Allocator, client: *lexe.LexeClient, args: ConfigureArgs, w: *std.Io.Writer, ew: *std.Io.Writer, json: bool) !void {
    // Resolve the BOLT12 offer: use --offer flag or fetch from Lexe node.
    var offer_buf: ?[]u8 = null;
    defer if (offer_buf) |buf| allocator.free(buf);

    const bolt12_offer: []const u8 = if (args.offer) |o|
        o
    else blk: {
        // Fetch offer from Lexe node via GET /v2/node/offer
        const result = fetchOffer(client) catch |err|
            exitErr(ew, "error: failed to fetch offer from Lexe node: {}\n", .{err});

        switch (result) {
            .ok => |resp| {
                offer_buf = allocator.dupe(u8, resp.value.offer) catch
                    exitErr(ew, "error: out of memory\n", .{});
                resp.deinit();
                break :blk offer_buf.?;
            },
            .err => |resp| handleApiError(ew, resp),
            .not_found => exitErr(ew, "error: Lexe node does not support offer retrieval (404). Pass --offer manually.\n", .{}),
        }
    };

    // Validate offer
    if (!ocean.validateBolt12Offer(allocator, bolt12_offer)) {
        exitErr(ew, "error: invalid BOLT12 offer (must start with 'lno1')\n", .{});
    }

    // The message is provided by the user (from the OCEAN web interface).
    const message = args.message orelse
        exitErr(ew, "error: --message is required. Copy the message from the OCEAN configuration page.\n", .{});

    if (json) {
        const output = ConfigureOutput{
            .message = message,
            .offer = bolt12_offer,
            .address = args.address,
        };
        try writeJson(allocator, w, output);
    } else {
        try w.print("OCEAN Lightning Payout Configuration\n", .{});
        try w.print("====================================\n\n", .{});
        try w.print("BOLT12 Offer:  {s}\n", .{bolt12_offer});
        if (args.address) |addr| {
            try w.print("BTC Address:   {s}\n", .{addr});
        }
        try w.print("\nMessage to sign:\n", .{});
        try w.print("  {s}\n", .{message});
        try w.print("\nSign this message with the private key of your OCEAN mining\n", .{});
        try w.print("address using BIP-322 or legacy Bitcoin message signing.\n", .{});
        try w.print("Paste the Base64 signature into the OCEAN web interface.\n", .{});
        try w.print("\nExample with bitcoin-cli:\n", .{});
        try w.print("  bitcoin-cli signmessage \"<address>\" \"{s}\"\n", .{message});
    }
}

const ConfigureOutput = struct {
    message: []const u8,
    offer: []const u8,
    address: ?[]const u8,
};

/// GET /v2/node/offer — fetch the BOLT12 offer from the Lexe node.
/// This endpoint may not be available on all sidecar versions.
const OfferResponse = struct {
    offer: []const u8,
};

fn fetchOffer(client: *lexe.LexeClient) lexe.ClientError!lexe.ApiResult(OfferResponse) {
    // Use the same HTTP internals pattern as the Lexe client.
    // We call GET /v2/node/offer on the sidecar.
    const base = std.mem.trimRight(u8, client.config.base_url, "/");
    var url_buf: [2048]u8 = undefined;
    const url = std.fmt.bufPrint(&url_buf, "{s}/v2/node/offer", .{base}) catch
        return error.InvalidUri;

    const uri = std.Uri.parse(url) catch return error.InvalidUri;

    const content_type_header = std.http.Header{
        .name = "Content-Type",
        .value = "application/json",
    };
    const extra_headers: []const std.http.Header = if (client.auth_header) |auth|
        &[_]std.http.Header{
            content_type_header,
            .{ .name = "Authorization", .value = auth },
        }
    else
        &[_]std.http.Header{content_type_header};

    var req = client.http_client.request(.GET, uri, .{
        .extra_headers = extra_headers,
    }) catch return error.ConnectionRefused;
    defer req.deinit();

    req.sendBodiless() catch return error.HttpRequestFailed;

    var redirect_buf: [8192]u8 = undefined;
    var response = req.receiveHead(&redirect_buf) catch return error.HttpRequestFailed;

    const status = response.head.status;
    const status_code: u10 = @intFromEnum(status);

    if (status == .not_found) {
        return .not_found;
    }

    const arena = client.allocator.create(std.heap.ArenaAllocator) catch
        return error.OutOfMemory;
    arena.* = std.heap.ArenaAllocator.init(client.allocator);
    errdefer {
        arena.deinit();
        client.allocator.destroy(arena);
    }
    const arena_alloc = arena.allocator();

    var transfer_buf: [8192]u8 = undefined;
    const reader = response.reader(&transfer_buf);
    const resp_body = reader.allocRemaining(arena_alloc, std.Io.Limit.limited(1024 * 1024)) catch
        return error.HttpRequestFailed;

    const is_success = status_code >= 200 and status_code < 300;

    if (!is_success) {
        const parsed = std.json.parseFromSlice(lexe.types.SdkError, arena_alloc, resp_body, .{
            .ignore_unknown_fields = true,
            .allocate = .alloc_always,
        }) catch return error.JsonParseFailed;
        return .{ .err = .{
            .value = parsed.value,
            .arena = arena,
        } };
    }

    const parsed = std.json.parseFromSlice(OfferResponse, arena_alloc, resp_body, .{
        .ignore_unknown_fields = true,
        .allocate = .alloc_always,
    }) catch return error.JsonParseFailed;
    return .{ .ok = .{
        .value = parsed.value,
        .arena = arena,
    } };
}

fn cmdInvoice(allocator: Allocator, client: *lexe.LexeClient, args: InvoiceArgs, w: *std.Io.Writer, ew: *std.Io.Writer, json: bool) !void {
    const result = client.createInvoice(.{
        .amount = args.amount,
        .description = args.description,
        .expiration_secs = 3600,
    }) catch |err| exitErr(ew, "error: {}\n", .{err});
    switch (result) {
        .ok => |resp| {
            defer resp.deinit();
            const inv = resp.value;
            if (json) {
                try writeJson(allocator, w, inv);
            } else {
                try w.print("Invoice:       {s}\n", .{inv.invoice});
                try w.print("Amount:        {s} sats\n", .{inv.amount orelse "amountless"});
                try w.print("Payment hash:  {s}\n", .{inv.payment_hash});
                try w.print("Expires at:    {d}\n", .{inv.expires_at});
            }
        },
        .err => |resp| handleApiError(ew, resp),
        .not_found => exitErr(ew, "error: unexpected 404\n", .{}),
    }
}

fn cmdPay(allocator: Allocator, client: *lexe.LexeClient, args: PayArgs, w: *std.Io.Writer, ew: *std.Io.Writer, json: bool) !void {
    const result = client.payInvoice(.{
        .invoice = args.bolt11,
    }) catch |err| exitErr(ew, "error: {}\n", .{err});
    switch (result) {
        .ok => |resp| {
            defer resp.deinit();
            const pay = resp.value;
            if (json) {
                try writeJson(allocator, w, pay);
            } else {
                try w.print("Payment sent.\n", .{});
                try w.print("Index:         {s}\n", .{pay.index});
                try w.print("Created at:    {d}\n", .{pay.created_at});
            }
        },
        .err => |resp| handleApiError(ew, resp),
        .not_found => exitErr(ew, "error: unexpected 404\n", .{}),
    }
}

fn cmdPayment(allocator: Allocator, client: *lexe.LexeClient, args: PaymentArgs, w: *std.Io.Writer, ew: *std.Io.Writer, json: bool) !void {
    const result = client.getPayment(args.index) catch |err| exitErr(ew, "error: {}\n", .{err});
    switch (result) {
        .ok => |resp| {
            defer resp.deinit();
            const p = resp.value;
            if (json) {
                try writeJson(allocator, w, p);
            } else {
                try w.print("Index:         {s}\n", .{p.index});
                try w.print("Rail:          {s}\n", .{p.rail.toString()});
                try w.print("Kind:          {s}\n", .{p.kind.toString()});
                try w.print("Direction:     {s}\n", .{p.direction.toString()});
                try w.print("Status:        {s}\n", .{p.status.toString()});
                try w.print("Status msg:    {s}\n", .{p.status_msg});
                if (p.amount) |amt| try w.print("Amount:        {s} sats\n", .{amt});
                try w.print("Fees:          {s} sats\n", .{p.fees});
                if (p.invoice) |inv| try w.print("Invoice:       {s}\n", .{inv});
                if (p.address) |addr| try w.print("Address:       {s}\n", .{addr});
                if (p.note) |note| try w.print("Note:          {s}\n", .{note});
                try w.print("Created at:    {d}\n", .{p.created_at});
                try w.print("Updated at:    {d}\n", .{p.updated_at});
                if (p.finalized_at) |ts| try w.print("Finalized at:  {d}\n", .{ts});
            }
        },
        .err => |resp| handleApiError(ew, resp),
        .not_found => exitErr(ew, "Payment not found.\n", .{}),
    }
}

fn cmdHealth(allocator: Allocator, client: *lexe.LexeClient, w: *std.Io.Writer, ew: *std.Io.Writer, json: bool) !void {
    const result = client.health() catch |err| exitErr(ew, "error: {}\n", .{err});
    switch (result) {
        .ok => |resp| {
            defer resp.deinit();
            if (json) {
                try writeJson(allocator, w, resp.value);
            } else {
                try w.print("Status: {s}\n", .{resp.value.status});
            }
        },
        .err => |resp| handleApiError(ew, resp),
        .not_found => exitErr(ew, "error: unexpected 404\n", .{}),
    }
}

// ── Main ─────────────────────────────────────────────────────────

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const args = try std.process.argsAlloc(allocator);
    defer std.process.argsFree(allocator, args);

    var out_buf: [4096]u8 = undefined;
    var w = std.fs.File.stdout().writer(&out_buf);
    const stdout = &w.interface;

    var err_buf: [4096]u8 = undefined;
    var ew = std.fs.File.stderr().writer(&err_buf);
    const stderr = &ew.interface;

    const cli = parseArgs(args) catch |err| {
        switch (err) {
            error.NoCommand => stderr.print("error: no command specified\n\n", .{}) catch {},
            error.UnknownCommand => stderr.print("error: unknown command\n\n", .{}) catch {},
            error.UnknownFlag => stderr.print("error: unknown flag\n\n", .{}) catch {},
            error.MissingFlagValue => stderr.print("error: missing value for flag\n\n", .{}) catch {},
            error.MissingAmount => stderr.print("error: invoice requires an amount\n\n", .{}) catch {},
            error.MissingInvoice => stderr.print("error: pay requires a BOLT11 invoice\n\n", .{}) catch {},
            error.MissingIndex => stderr.print("error: payment requires an index\n\n", .{}) catch {},
        }
        printUsage(stderr) catch {};
        stderr.flush() catch {};
        std.process.exit(1);
    };

    switch (cli.command) {
        .help => {
            try printUsage(stdout);
            try stdout.flush();
            return;
        },
        .sign => |a| {
            // Sign command doesn't need Lexe client
            try cmdSign(allocator, a, stdout, stderr, cli.json);
            try stdout.flush();
            return;
        },
        else => {},
    }

    var client = lexe.LexeClient.init(allocator, .{
        .base_url = cli.url,
        .credentials = cli.credentials,
    }) catch |err| {
        stderr.print("error: failed to initialize client: {}\n", .{err}) catch {};
        stderr.flush() catch {};
        std.process.exit(1);
    };
    defer client.deinit();

    switch (cli.command) {
        .info => try cmdInfo(allocator, &client, stdout, stderr, cli.json),
        .configure => |a| try cmdConfigure(allocator, &client, a, stdout, stderr, cli.json),
        .invoice => |a| try cmdInvoice(allocator, &client, a, stdout, stderr, cli.json),
        .pay => |a| try cmdPay(allocator, &client, a, stdout, stderr, cli.json),
        .payment => |a| try cmdPayment(allocator, &client, a, stdout, stderr, cli.json),
        .health => try cmdHealth(allocator, &client, stdout, stderr, cli.json),
        .sign => unreachable, // handled above
        .help => unreachable,
    }

    try stdout.flush();
}
