//! Binary-level smoke tests.
//!
//! These drive the COMPILED `oceanln` binary via `assert_cmd`, exercising the
//! end-to-end CLI surface (argument parsing, stdout/stderr contracts, exit
//! codes, and the sidecar HTTP round-trip) without reaching into the library.
//!
//! The success path stands up a deliberately tiny, std-only mock sidecar: a
//! blocking `TcpListener` on its own thread that speaks just enough HTTP to
//! answer one `create_offer` call. Keeping it sync (no tokio/axum) means it
//! composes cleanly with `assert_cmd`'s blocking process spawn.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use assert_cmd::Command;
use predicates::prelude::*;

/// A valid 24-word BIP39 mnemonic whose BIP84 `m/84'/0'/0'/0/0` address is
/// `bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r`.
const TEST_MNEMONIC: &str = "music mystery deliver gospel profit blanket leaf tell photo \
segment letter degree nice plastic duty canyon mammal marble bicycle economy unique find \
cream dune";

/// A syntactically valid BOLT12 offer the mock sidecar hands back.
const MOCK_OFFER: &str = "lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9uve2rlrm9deu7xyfzrcgqp0s";

fn bin() -> Command {
    Command::cargo_bin("oceanln").expect("binary `oceanln` should build")
}

/// A localhost URL whose connect is refused deterministically. Port 1 (tcpmux)
/// is never bound by a normal process, so this avoids the ephemeral-port-reuse
/// race that a `bind(0)`-then-`drop` helper hits under parallel test runs.
const REFUSED_URL: &str = "http://127.0.0.1:1";

/// Stand up a one-shot mock sidecar on an ephemeral port.
///
/// Returns the bound port. A background thread accepts a single connection,
/// drains the HTTP request, and replies with a fixed `create_offer` body.
/// Read timeouts guard against a client that connects but never finishes its
/// request, so a misbehaving run fails fast instead of hanging the suite.
fn spawn_mock_sidecar() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock sidecar");
    let port = listener.local_addr().expect("local addr").port();

    std::thread::spawn(move || {
        // Accept in a loop: a probing/aborted connection shouldn't consume the
        // one real request we care about. Bail after the first fully served one.
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            stream.set_read_timeout(Some(Duration::from_secs(5))).ok();

            // Read until we've seen the end of the HTTP headers. We don't need
            // the body — the request has no semantic effect on the canned reply.
            let mut buf = Vec::new();
            let mut chunk = [0u8; 1024];
            loop {
                match stream.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        buf.extend_from_slice(&chunk[..n]);
                        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            let body = format!(r#"{{"offer":"{MOCK_OFFER}"}}"#);
            let response = format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\
                 \r\n\
                 {}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
            // Served the real request; the test is done with us.
            return;
        }
    });

    port
}

#[test]
fn generate_prints_24_words() {
    let out = bin().arg("generate").assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout");
    let words: Vec<&str> = stdout.split_whitespace().collect();
    assert_eq!(
        words.len(),
        24,
        "generate stdout should be 24 words: {stdout:?}"
    );
}

#[test]
fn generate_json_has_24_word_mnemonic() {
    let out = bin().args(["generate", "--json"]).assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout");
    let v: serde_json::Value =
        serde_json::from_str(&stdout).expect("generate --json stdout should be valid JSON");
    let mnemonic = v
        .get("mnemonic")
        .and_then(|m| m.as_str())
        .expect("`mnemonic` field should be a string");
    assert_eq!(
        mnemonic.split_whitespace().count(),
        24,
        "mnemonic should have 24 words"
    );
}

#[test]
fn version_flag_succeeds() {
    bin().arg("--version").assert().success();
}

#[test]
fn help_mentions_both_subcommands() {
    bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("generate").and(predicate::str::contains("payout")));
}

#[test]
fn payout_without_message_is_usage_error() {
    // clap surfaces a missing required `--message` as exit code 2.
    bin()
        .args(["payout", "--description", "smoke"])
        .assert()
        .code(2);
}

#[test]
fn payout_against_dead_sidecar_reports_unreachable() {
    bin()
        .args([
            "payout",
            "--url",
            REFUSED_URL,
            "--message",
            "Configure OCEAN payout at block 840000",
            "--description",
            "smoke",
        ])
        .write_stdin(TEST_MNEMONIC)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("could not reach sidecar"));
}

#[test]
fn payout_success_path_signs_and_derives_address() {
    let port = spawn_mock_sidecar();
    let url = format!("http://127.0.0.1:{port}");

    let out = bin()
        .args([
            "payout",
            "--url",
            &url,
            "--json",
            "--message",
            "Configure OCEAN payout at block 840000",
            "--description",
            "smoke",
        ])
        .write_stdin(TEST_MNEMONIC)
        .assert()
        .success();

    let stdout = String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout");
    let v: serde_json::Value =
        serde_json::from_str(&stdout).expect("payout --json stdout should be valid JSON");

    let address = v
        .get("address")
        .and_then(|a| a.as_str())
        .expect("`address` field should be a string");
    assert!(
        address.starts_with("bc1q"),
        "address should be a mainnet bech32 address: {address}"
    );

    let signature = v
        .get("signature")
        .and_then(|s| s.as_str())
        .expect("`signature` field should be a string");
    assert!(!signature.is_empty(), "signature should be non-empty");
}

#[test]
fn payout_with_existing_offer_signs_offline() {
    // `--offer` must NOT contact the sidecar: point `--url` at a refused port
    // to prove it. The flow should still succeed, echo the provided offer back,
    // and produce a signature + address.
    let out = bin()
        .args([
            "payout",
            "--url",
            REFUSED_URL,
            "--json",
            "--offer",
            MOCK_OFFER,
            "--message",
            "Configure OCEAN payout at block 840000",
        ])
        .write_stdin(TEST_MNEMONIC)
        .assert()
        .success();

    let stdout = String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout");
    let v: serde_json::Value =
        serde_json::from_str(&stdout).expect("payout --json stdout should be valid JSON");

    assert_eq!(
        v.get("offer").and_then(|o| o.as_str()),
        Some(MOCK_OFFER),
        "the provided offer should be echoed back unchanged"
    );
    assert!(v
        .get("address")
        .and_then(|a| a.as_str())
        .is_some_and(|a| a.starts_with("bc1q")));
    assert!(v
        .get("signature")
        .and_then(|s| s.as_str())
        .is_some_and(|s| !s.is_empty()));
}

#[test]
fn payout_offer_conflicts_with_description() {
    // --offer and --description are mutually exclusive (one creates, one reuses).
    bin()
        .args([
            "payout",
            "--offer",
            MOCK_OFFER,
            "--description",
            "smoke",
            "--message",
            "x",
        ])
        .assert()
        .code(2);
}
