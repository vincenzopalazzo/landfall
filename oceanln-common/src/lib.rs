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
pub mod seed;
pub mod sign;
