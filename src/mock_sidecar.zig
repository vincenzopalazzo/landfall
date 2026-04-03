//! Mock Lexe Sidecar HTTP Server
//!
//! Serves canned JSON responses on localhost for integration testing.
//! Supports: /v2/health, /v2/node/node_info, /v2/node/offer
const std = @import("std");
const Allocator = std.mem.Allocator;

pub const MockSidecar = struct {
    thread: ?std.Thread = null,
    running: std.atomic.Value(bool) = std.atomic.Value(bool).init(false),
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

    pub fn init(port: u16) MockSidecar {
        return .{ .port = port };
    }

    pub fn start(self: *MockSidecar) !void {
        self.running.store(true, .release);
        self.thread = try std.Thread.spawn(.{}, serve, .{self});
    }

    pub fn stop(self: *MockSidecar) void {
        self.running.store(false, .release);
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
        var server = std.net.StreamServer.init(.{ .reuse_address = true }) catch return;
        defer server.deinit();
        server.listen(addr) catch return;

        while (self.running.load(.acquire)) {
            // Use a short timeout so we can check the running flag
            const conn = server.accept(10_000_000) catch |err| { // 10ms timeout
                if (err == error.Timeout or err == error.WouldBlock) continue;
                continue;
            };
            handleConnection(conn) catch {};
        }
    }

    fn handleConnection(conn: std.net.StreamServer.Connection) !void {
        defer conn.stream.close();

        var buf: [4096]u8 = undefined;
        const n = conn.stream.read(&buf) catch return;
        if (n == 0) return;

        const request = buf[0..n];

        // Parse the request line to get the path
        const path = extractPath(request) orelse return;

        const response_body = if (std.mem.startsWith(u8, path, "/v2/health"))
            health_json
        else if (std.mem.startsWith(u8, path, "/v2/node/node_info"))
            node_info_json
        else if (std.mem.startsWith(u8, path, "/v2/node/offer"))
            offer_json
        else
            null;

        if (response_body) |body| {
            var resp_buf: [4096]u8 = undefined;
            const resp = std.fmt.bufPrint(&resp_buf,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {d}\r\n\r\n{s}",
                .{ body.len, body },
            ) catch return;
            _ = conn.stream.write(resp) catch {};
        } else {
            const not_found = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
            _ = conn.stream.write(not_found) catch {};
        }
    }

    fn extractPath(request: []const u8) ?[]const u8 {
        // Find first space (after method)
        const space1 = std.mem.indexOf(u8, request, " ") orelse return null;
        const rest = request[space1 + 1 ..];
        // Find second space (before HTTP version)
        const space2 = std.mem.indexOf(u8, rest, " ") orelse return null;
        return rest[0..space2];
    }
};

test "MockSidecar extractPath" {
    const path = MockSidecar.extractPath("GET /v2/health HTTP/1.1\r\n");
    try std.testing.expect(path != null);
    try std.testing.expectEqualStrings("/v2/health", path.?);
}
