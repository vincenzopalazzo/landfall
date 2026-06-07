//! oceanln-common — shared core for the OCEAN Lightning payout tooling.
//!
//! Both the `oceanln` CLI (`oceanln-cli`) and the local HTTP server
//! (`oceanln-httpd`) are thin frontends over this crate. It owns:
//! - [`sign`] — BIP-322 message signing + BIP84 address derivation,
//! - [`seed`] — [`seed::SeedSource`], how a frontend obtains the mnemonic
//!   without it ever crossing a transport boundary,
//! - [`client`] — the Lexe sidecar HTTP client,
//! - [`lexe_wallet`] — in-process Lexe wallet ops (feature `lexe-sdk`),
//! - [`error`] — the shared error/result types.

pub mod client;
pub mod error;
#[cfg(feature = "lexe-sdk")]
pub mod lexe_wallet;
/// Shared network helpers (host-loopback check, etc.) used by both
/// transport crates so DNS-rebinding defense doesn't drift between them.
pub mod net;
pub mod ocean;
pub mod seed;
/// Transport-agnostic orchestration shared by every frontend
/// (oceanln-httpd, oceanln-cli, the Tauri desktop shell, oceanln-mcp).
pub mod service;
pub mod sign;
/// The [`wallet_provider::WalletProvider`] trait + production
/// [`wallet_provider::LexeWalletProvider`]. Lives here so multiple
/// transport crates can implement against the same abstraction without
/// depending on each other. Gated on `lexe-sdk` because the trait
/// references [`lexe_wallet::OceanPayout`] in its method signatures.
#[cfg(feature = "lexe-sdk")]
pub mod wallet_provider;
