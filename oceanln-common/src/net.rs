//! Tiny network-related helpers shared by `oceanln-httpd` and
//! `oceanln-mcp`. Both crates need the same Host-header validation
//! (DNS-rebinding defense for loopback-bound listeners); having it
//! here keeps the two implementations from drifting.

/// True if `host` (a `Host` header value, possibly `host:port`) names the
/// local machine. Anything else is rejected to blunt DNS-rebinding
/// attacks that resolve an attacker-controlled name to `127.0.0.1`.
///
/// A prefix test like `starts_with("127.")` would wrongly accept names
/// such as `127.evil.com` that resolve off-loopback — so we require the
/// hostname to be the literal `localhost` or to parse as a loopback IP.
pub fn host_is_loopback(host: &str) -> bool {
    let hostname = if let Some(rest) = host.strip_prefix('[') {
        // Bracketed IPv6 literal: take up to the closing bracket.
        rest.split(']').next().unwrap_or(rest)
    } else if host.matches(':').count() > 1 {
        // Bare IPv6 (e.g. `::1`) — multiple colons, no brackets, no port.
        host
    } else {
        host.split(':').next().unwrap_or(host)
    };
    if hostname == "localhost" {
        return true;
    }
    hostname
        .parse::<std::net::IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_names_and_ips_pass() {
        for ok in [
            "localhost",
            "localhost:7762",
            "127.0.0.1",
            "127.0.0.1:7762",
            "127.0.0.2", // all of 127.0.0.0/8 is loopback
            "::1",
            "[::1]:7762",
        ] {
            assert!(host_is_loopback(ok), "{ok} should be loopback");
        }
    }

    #[test]
    fn routable_or_spoofed_names_fail() {
        // The bug this defends against: a name that merely *starts with*
        // "127." must not pass, nor any routable host. DNS rebinding
        // can point any name at a loopback IP, but the Host header
        // reflects the original name the browser typed.
        for bad in [
            "127.evil.com",
            "127.0.0.1.evil.com",
            "evil.com",
            "192.168.1.1",
            "10.0.0.1",
            "8.8.8.8",
        ] {
            assert!(!host_is_loopback(bad), "{bad} should NOT be loopback");
        }
    }
}
