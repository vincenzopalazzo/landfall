//! Mock Lexe Sidecar HTTP Server
//!
//! Serves canned JSON responses on localhost for integration testing.
//! Supports: /v2/health, /v2/node/node_info, /v2/node/offer,
//!           /v2/node/create_invoice (POST), /v2/node/payment (GET)
const std = @import("std");
const Allocator = std.mem.Allocator;

pub const MockSidecar = struct {
    thread: ?std.Thread = null,
    running: std.atomic.Value(bool) = std.atomic.Value(bool).init(false),
    server: ?std.net.Server = null,
    port: u16,

    const node_info_json =
        \\{"version":"0.9.2-mock","measurement":"aabb","user_pk":"cc","node_pk":"dd",
        \\"balance":"50000","lightning_balance":"25000","lightning_sendable_balance":"20000",
        \\"lightning_max_sendable_balance":"25000","onchain_balance":"25000",
        \\"onchain_trusted_balance":"25000","num_channels":2,"num_usable_channels":2}
    ;

    const health_json =
        \\{"status":"ok"}
    ;

    const offer_json =
        \\{"offer":"lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9uve2rlrm9deu7xyfzrcgqp0s"}
    ;

    const create_invoice_json =
        \\{"index":"0000001772349163844-ln_mock","invoice":"lnbc10n1pnmockqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqf",
        \\"description":"mock invoice","amount":"1000","created_at":1772349163844,
        \\"expires_at":1772352763844,"payment_hash":"aabbccdd00112233aabbccdd00112233aabbccdd00112233aabbccdd00112233",
        \\"payment_secret":"11223344556677881122334455667788112233445566778811223344556677aa"}
    ;

    const payment_json =
        \\{"index":"0000001772349163844-ln_mock","rail":"invoice","kind":"invoice",
        \\"direction":"inbound","amount":"1000","fees":"0","status":"completed",
        \\"status_msg":"received","created_at":1772349163844,"updated_at":1772349170000}
    ;

    pub fn init(port: u16) MockSidecar {
        return .{ .port = port };
    }

    pub fn start(self: *MockSidecar) !void {
        self.running.store(true, .release);
        self.thread = try std.Thread.spawn(.{}, serve, .{self});
    }

    pub fn stop(self: *MockSidecar) void {
        self.running.store(false, .release);
        // Close the server socket to unblock accept()
        if (self.server) |*s| {
            s.deinit();
            self.server = null;
        }
        if (self.thread) |t| {
            t.join();
            self.thread = null;
        }
    }

    pub fn baseUrl(self: *const MockSidecar, buf: []u8) []const u8 {
        return std.fmt.bufPrint(buf, "http://127.0.0.1:{d}", .{self.port}) catch "http://127.0.0.1:15393";
    }

    fn serve(self: *MockSidecar) void {
        const addr = std.net.Address.parseIp4("127.0.0.1", self.port) catch return;
        self.server = addr.listen(.{ .reuse_address = true }) catch return;

        while (self.running.load(.acquire)) {
            if (self.server) |*s| {
                const conn = s.accept() catch break;
                handleConnection(conn) catch {};
            } else break;
        }
    }

    fn handleConnection(conn: std.net.Server.Connection) !void {
        defer conn.stream.close();

        var buf: [8192]u8 = undefined;
        const n = conn.stream.read(&buf) catch return;
        if (n == 0) return;

        const request = buf[0..n];
        const path = extractPath(request) orelse return;

        const response_body: ?[]const u8 = if (std.mem.startsWith(u8, path, "/v2/health"))
            health_json
        else if (std.mem.startsWith(u8, path, "/v2/node/node_info"))
            node_info_json
        else if (std.mem.startsWith(u8, path, "/v2/node/offer"))
            offer_json
        else if (std.mem.startsWith(u8, path, "/v2/node/create_invoice"))
            create_invoice_json
        else if (std.mem.startsWith(u8, path, "/v2/node/payment"))
            payment_json
        else
            null;

        if (response_body) |body| {
            var resp_buf: [4096]u8 = undefined;
            const resp = std.fmt.bufPrint(&resp_buf,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {d}\r\nConnection: close\r\n\r\n{s}",
                .{ body.len, body },
            ) catch return;
            _ = conn.stream.write(resp) catch {};
        } else {
            const not_found = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            _ = conn.stream.write(not_found) catch {};
        }
    }

    fn extractPath(request: []const u8) ?[]const u8 {
        const space1 = std.mem.indexOf(u8, request, " ") orelse return null;
        const rest = request[space1 + 1 ..];
        const space2 = std.mem.indexOf(u8, rest, " ") orelse return null;
        return rest[0..space2];
    }
};

test "MockSidecar extractPath" {
    const path = MockSidecar.extractPath("GET /v2/health HTTP/1.1\r\n");
    try std.testing.expect(path != null);
    try std.testing.expectEqualStrings("/v2/health", path.?);
}

test "MockSidecar extractPath POST" {
    const path = MockSidecar.extractPath("POST /v2/node/create_invoice HTTP/1.1\r\n");
    try std.testing.expect(path != null);
    try std.testing.expectEqualStrings("/v2/node/create_invoice", path.?);
}
