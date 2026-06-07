//! oceanln-mcp binary — read-only MCP server in front of `oceanln-httpd`.
//!
//! Run it next to a running `oceanln-httpd` on the same host. The MCP
//! client (Goose, Claude Code, …) connects to this binary's port; this
//! binary forwards every tool call to httpd as a REST request. There is
//! no shared process state, no shared seed access — the only thing
//! crossing the boundary is HTTP.

use std::net::SocketAddr;

use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use clap::Parser;
use oceanln_common::net::host_is_loopback;
use oceanln_mcp::{streamable_http_service, HttpdClient};

// No `Debug`: `Cli` holds `--httpd-token`, and we don't want a stray
// `{:?}` to leak it.
#[derive(Parser)]
#[command(
    name = "oceanln-mcp",
    version,
    about = "Read-only MCP server that proxies OCEAN payout state from a running oceanln-httpd"
)]
struct Cli {
    /// Base URL of the upstream `oceanln-httpd`. Defaults to the
    /// same loopback port `oceanln-httpd` listens on by default.
    #[arg(long, default_value = "http://127.0.0.1:7762")]
    base: String,

    /// Bearer token for the upstream httpd. Omit when httpd was
    /// started with `--no-auth` (v1 single-host deploy).
    #[arg(long)]
    httpd_token: Option<String>,

    /// Loopback address THIS server binds for incoming MCP clients.
    /// Must be a loopback IP (127.0.0.0/8 or ::1) — refusing routable
    /// binds matches the `oceanln-httpd` posture.
    #[arg(long, default_value = "127.0.0.1:7763")]
    bind: SocketAddr,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if !cli.bind.ip().is_loopback() {
        eprintln!(
            "error: --bind {} is not a loopback address; this server only serves 127.0.0.1/::1",
            cli.bind
        );
        std::process::exit(1);
    }

    let client = HttpdClient::new(&cli.base, cli.httpd_token.clone());
    let service = streamable_http_service(client);

    // Wrap the MCP route in a guard that defends two browser-driven
    // attack vectors that loopback-binding alone DOES NOT stop:
    //   - DNS rebinding (`attacker.com` resolves to 127.0.0.1 after the
    //     page loads): defeated by the Host check, since the browser
    //     uses the original hostname for the `Host` header.
    //   - CSRF from a same-machine browser page: defeated by refusing
    //     any request that carries an `Origin` header. Native MCP
    //     clients (Goose, Claude Code) send no `Origin`; a browser
    //     always does. (No legitimate browser MCP client exists in v1.)
    let app = axum::Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn(loopback_guard));
    let listener = match tokio::net::TcpListener::bind(cli.bind).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("error: could not bind {}: {e}", cli.bind);
            std::process::exit(1);
        }
    };

    eprintln!(
        "oceanln-mcp listening on http://{}/mcp — proxying to {}",
        cli.bind, cli.base
    );
    if cli.httpd_token.is_none() {
        eprintln!("upstream auth: NONE (no --httpd-token). Assuming `oceanln-httpd --no-auth`.");
    } else {
        eprintln!("upstream auth: using --httpd-token (not echoed)");
    }
    eprintln!("MCP add command (Goose / Claude Code):");
    eprintln!("  URL:    http://{}/mcp", cli.bind);
    eprintln!("  Header: (none required — this server is unauthenticated)");

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
    {
        eprintln!("error: server stopped: {e}");
        std::process::exit(1);
    }
}

/// Reject requests whose `Host` header isn't a loopback name/IP (DNS
/// rebinding) OR that carry any `Origin` header (cross-origin browser
/// request). Same posture as `oceanln-httpd`'s `guard` middleware,
/// minus the bearer check (MCP itself is intentionally unauthed in v1
/// — the auth lives between us and httpd via `--httpd-token`).
async fn loopback_guard(req: Request, next: Next) -> Response {
    let headers = req.headers();
    if let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) {
        if !host_is_loopback(host) {
            return (StatusCode::FORBIDDEN, "host not allowed").into_response();
        }
    }
    if headers.contains_key(header::ORIGIN) {
        // Native MCP clients don't set `Origin`; a non-empty value means
        // a browser is calling us, which we don't support in v1.
        return (StatusCode::FORBIDDEN, "origin not allowed").into_response();
    }
    next.run(req).await
}
