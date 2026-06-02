//! oceanln — OCEAN Lightning payout CLI library.
//!
//! The binary in [`src/main.rs`](../src/main.rs) is a thin wrapper.
//! Integration tests live in `tests/` and consume this library
//! directly so they can drive the sidecar HTTP client against an
//! in-process mock.

pub mod cli;
pub mod client;
pub mod error;
pub mod sign;
