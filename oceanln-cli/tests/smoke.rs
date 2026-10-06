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

// `init` only exists in the in-process build (the default). `--dry-run` skips
// provisioning, so these run offline with no Lexe backend.
#[cfg(feature = "lexe-sdk")]
#[test]
fn init_generate_dry_run_derives_seed_and_address() {
    let out = bin()
        .args(["init", "--generate", "--dry-run", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout");
    let v: serde_json::Value =
        serde_json::from_str(&stdout).expect("init --json stdout should be valid JSON");
    assert_eq!(
        v["mnemonic"].as_str().map(|m| m.split_whitespace().count()),
        Some(24),
        "a freshly generated 24-word seed"
    );
    assert!(v["mining_address"]
        .as_str()
        .is_some_and(|a| a.starts_with("bc1q")));
    assert_eq!(v["provisioned"], serde_json::json!(false), "dry run");
}

#[cfg(feature = "lexe-sdk")]
#[test]
fn init_dry_run_with_seed_omits_mnemonic_and_derives_known_address() {
    let out = bin()
        .args(["init", "--dry-run", "--json"])
        .write_stdin(TEST_MNEMONIC)
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    // A provided seed must NOT be echoed back.
    assert!(v.get("mnemonic").is_none(), "must not echo a provided seed");
    assert_eq!(
        v["mining_address"].as_str(),
        Some("bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r"),
        "deterministic BIP84 address for TEST_MNEMONIC"
    );
    assert_eq!(v["provisioned"], serde_json::json!(false));
}

// `--dry-run` is a pure preview: it must NOT persist a seed file (no side
// effects). We point `--seed-file` at an isolated temp path and assert the
// file never appears, and that the JSON omits `seed_file`.
#[cfg(feature = "lexe-sdk")]
#[test]
fn init_dry_run_does_not_persist_seed_file() {
    let seed_path = std::env::temp_dir().join(format!(
        "oceanln-smoke-dryrun-{}-{}.seed",
        std::process::id(),
        line!()
    ));
    std::fs::remove_file(&seed_path).ok(); // ensure clean slate

    let out = bin()
        .args([
            "init",
            "--generate",
            "--dry-run",
            "--json",
            "--seed-file",
            seed_path.to_str().unwrap(),
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    assert!(
        !seed_path.exists(),
        "dry-run must not write a seed file at {}",
        seed_path.display()
    );
    assert!(
        v.get("seed_file").is_none(),
        "dry-run JSON must omit `seed_file`"
    );

    std::fs::remove_file(&seed_path).ok();
}

// A generated seed must be shown to the user BEFORE the fallible persist step,
// so a store conflict can never lose words the user hasn't seen. We point
// `--seed-file` at an existing file holding a *different* seed (no `--force`):
// `store_seed` will reject it, but the freshly generated 24 words must already
// be on stdout by then.
#[cfg(feature = "lexe-sdk")]
#[test]
fn init_generate_surfaces_seed_before_store_conflict() {
    let dir = std::env::temp_dir().join(format!(
        "oceanln-smoke-conflict-{}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let seed_path = dir.join("seed");
    // A different, valid seed already persisted at the target.
    std::fs::write(&seed_path, TEST_MNEMONIC).unwrap();

    let out = bin()
        .args([
            "init",
            "--generate",
            "--seed-file",
            seed_path.to_str().unwrap(),
        ])
        .assert()
        .failure(); // store conflict aborts before provisioning
    let stdout = String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout");

    // The generated seed line must have printed despite the later failure.
    let seed_line = stdout
        .lines()
        .find(|l| l.starts_with("Seed:"))
        .expect("generated seed must be surfaced before the store conflict");
    let words = seed_line
        .trim_start_matches("Seed:")
        .split_whitespace()
        .count();
    assert_eq!(words, 24, "surfaced seed should be 24 words: {seed_line:?}");

    std::fs::remove_dir_all(&dir).ok();
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
fn help_lists_every_subcommand() {
    bin().arg("--help").assert().success().stdout(
        predicate::str::contains("generate")
            .and(predicate::str::contains("payout"))
            .and(predicate::str::contains("verify")),
    );
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
    // and produce a signature + address. The message embeds the offer (as the
    // real OCEAN message does), which the offer/message consistency guard
    // requires.
    let message = format!("Configure OCEAN payout to {MOCK_OFFER} at block 840000");
    let out = bin()
        .args([
            "payout",
            "--url",
            REFUSED_URL,
            "--json",
            "--offer",
            MOCK_OFFER,
            "--message",
            &message,
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

/// `verify` is the offline counterpart of `payout`: the signature `payout`
/// prints must verify for the address it prints, over the exact message —
/// and must NOT verify once any of the three is changed. This is the
/// end-to-end guard for the BIP-322 key/address binding.
#[test]
fn verify_round_trips_payout_signature_and_rejects_tampering() {
    let message = format!("Configure OCEAN payout to {MOCK_OFFER} at block 840000");
    let out = bin()
        .args([
            "payout",
            "--url",
            REFUSED_URL,
            "--json",
            "--offer",
            MOCK_OFFER,
            "--message",
            &message,
        ])
        .write_stdin(TEST_MNEMONIC)
        .assert()
        .success();
    let v: serde_json::Value =
        serde_json::from_slice(&out.get_output().stdout).expect("payout --json");
    let address = v["address"].as_str().unwrap().to_string();
    let signature = v["signature"].as_str().unwrap().to_string();
    assert_eq!(address, "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r");

    // Valid: exit 0, JSON says so, and nothing is read from stdin.
    let ok = bin()
        .args([
            "verify",
            "--json",
            "--address",
            &address,
            "--message",
            &message,
            "--signature",
            &signature,
        ])
        .assert()
        .success();
    let v: serde_json::Value = serde_json::from_slice(&ok.get_output().stdout).unwrap();
    assert_eq!(v["valid"], true);
    assert!(v.get("reason").is_none());

    // Tampered message → exit 1 with a reason.
    let tampered = format!("{message} at block 840001");
    let bad = bin()
        .args([
            "verify",
            "--json",
            "--address",
            &address,
            "--message",
            &tampered,
            "--signature",
            &signature,
        ])
        .assert()
        .code(1);
    let v: serde_json::Value = serde_json::from_slice(&bad.get_output().stdout).unwrap();
    assert_eq!(v["valid"], false);
    assert!(v["reason"].as_str().is_some_and(|r| !r.is_empty()));

    // Same signature presented for an address our key does not control →
    // exit 1. (BIP-84 vector address, a different key.)
    bin()
        .args([
            "verify",
            "--address",
            "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu",
            "--message",
            &message,
            "--signature",
            &signature,
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("INVALID"));

    // Garbage signature → exit 1, no panic.
    bin()
        .args([
            "verify",
            "--address",
            &address,
            "--message",
            &message,
            "--signature",
            "not-a-signature",
        ])
        .assert()
        .code(1);
}

/// QA-011: a seed piped with one word per line (`cat seedfile | oceanln …`)
/// must be read whole, exactly like a seed file is — not just its first line.
#[test]
fn payout_accepts_seed_piped_one_word_per_line() {
    let message = format!("Configure OCEAN payout to {MOCK_OFFER} at block 840000");
    let out = bin()
        .args([
            "payout",
            "--url",
            REFUSED_URL,
            "--json",
            "--offer",
            MOCK_OFFER,
            "--message",
            &message,
        ])
        .write_stdin(format!("{}\n", TEST_MNEMONIC.replace(' ', "\n")))
        .assert()
        .success();
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert_eq!(v["address"], "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r");
}

#[test]
fn payout_signs_ocean_json_message() {
    // OCEAN's config message is a JSON blob embedding the BOLT12 offer. It must
    // be signed verbatim as an opaque string. Using the embedded offer as
    // --offer keeps this fully offline (no sidecar). Offer + message from a
    // real OCEAN-style example.
    const OCEAN_OFFER: &str = "lno1pg7y7s69g98zq5rp09hh2arnypnx7u3qvf3nzufswdckcwtg095rqer88pervem8d4skkmngdse8gatgv5u8qan6ddm8gervwqepp6qrzjnh2g73mj79mdtqs8kuhsj2hqstxh358fkxw6ghdhns0stc63ts9kgyekcajjz9kgnzhsmmcmsz38c0h2d24hujg29zfnnj74a0a0t8qgpf47elgqaacqpscdz78799k4lt5uv8ve3x5hsg0znwzw3vfsskeqcqxwrxyu7xkttmnqqhnkrj5d22efa40sh2u3vtw2f9k0m4jfup93cv3rpq45hwj84s9c0ds0ta6gguf8adymtqx03wg7rjkgnn0c896lsxh2w4vc7n4xcdvl3xnwtxgqfz5svnw0z2qqkxwla3f0p6gxquq68x22567atpzhxqh8quyj3slgsuc3w4kykjufzp52dwp5x65alhdcgjavfqsmr90pjjuctswqtzzqhaklykhzhm6sn00ek6vwkp24ayh6q7ux83cnrw2znwayfmwszxns";
    let message = format!(r#"{{"height":944040,"lightning_bolt12":"{OCEAN_OFFER}"}}"#);

    let out = bin()
        .args([
            "payout",
            "--url",
            REFUSED_URL,
            "--json",
            "--offer",
            OCEAN_OFFER,
            "--message",
            &message,
        ])
        .write_stdin(TEST_MNEMONIC)
        .assert()
        .success();

    let stdout = String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout");
    let v: serde_json::Value =
        serde_json::from_str(&stdout).expect("payout --json stdout should be valid JSON");

    // The JSON message must be signed verbatim (echoed back unchanged).
    assert_eq!(
        v.get("message").and_then(|m| m.as_str()),
        Some(message.as_str()),
        "the OCEAN JSON message must round-trip unchanged"
    );
    assert_eq!(v.get("offer").and_then(|o| o.as_str()), Some(OCEAN_OFFER));
    assert!(v
        .get("signature")
        .and_then(|s| s.as_str())
        .is_some_and(|s| !s.is_empty()));
    assert!(v
        .get("address")
        .and_then(|a| a.as_str())
        .is_some_and(|a| a.starts_with("bc1q")));
}

#[test]
fn payout_rejects_offer_not_in_message() {
    // A --offer that isn't embedded in --message would sign for a different
    // offer than the one shown — must be rejected before any signing.
    bin()
        .args([
            "payout",
            "--url",
            REFUSED_URL,
            "--offer",
            MOCK_OFFER,
            "--message",
            "Configure OCEAN payout to lno1somethingelse at block 840000",
        ])
        .write_stdin(TEST_MNEMONIC)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("not present in --message"));
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
