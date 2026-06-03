//! Where a frontend obtains the user's mnemonic.
//!
//! The seed must never cross a transport boundary (HTTP body, query string,
//! etc.). Instead, `oceanln-httpd` is configured with a [`SeedSource`] and
//! resolves the mnemonic *locally, in-process* for each request that needs to
//! sign or touch the wallet. The 24 words go from this source straight into a
//! [`MnemonicSecret`] (zeroed on drop) and are never echoed back to the caller.

use std::path::PathBuf;

use crate::error::Result;
use crate::sign::{parse_mnemonic, resolve_seed, MnemonicSecret};

/// A local source the server reads the mnemonic from.
///
/// Today only [`SeedSource::File`] is supported. Keychain / interactive-unlock
/// backends can be added as further variants without changing call sites.
#[derive(Debug, Clone)]
pub enum SeedSource {
    /// A local file containing the 24 words (one line, or whitespace/newline
    /// separated). The file lives outside the transport — the operator places
    /// it; the server only reads it.
    File(PathBuf),
}

impl SeedSource {
    /// Resolve the mnemonic, validating it is a well-formed 24-word BIP39
    /// phrase. Reading is delegated to [`resolve_seed`] so the server inherits
    /// the same `0600` owner-only permission check the CLI enforces (rejecting
    /// a group/world-readable seed file) and we keep a single seed-file reader.
    pub fn load(&self) -> Result<MnemonicSecret> {
        match self {
            SeedSource::File(path) => {
                let secret = resolve_seed(Some(path))?;
                // Enforce the 24-word count before handing the secret to a
                // signer, so a malformed seed file fails loudly here.
                parse_mnemonic(&secret)?;
                Ok(secret)
            }
        }
    }
}
