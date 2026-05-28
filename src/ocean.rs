//! BOLT12 offer + OCEAN configuration validation.
//!
//! Mirrors the contract of `src/ocean.zig` in the previous Zig
//! implementation: full structural validation via `lightning::offers::offer::Offer`
//! (bech32, TLV, point-on-curve), not a substring check.

use crate::error::{Error, Result};
use lightning::offers::offer::Offer;
use std::str::FromStr;

/// Validate a BOLT12 offer by fully decoding it.
pub fn validate_bolt12_offer(s: &str) -> Result<()> {
    Offer::from_str(s).map_err(|e| Error::InvalidOffer(format!("{e:?}")))?;
    Ok(())
}

/// Block height: either "latest" or a non-empty ASCII numeric string.
///
/// Kept for parity with the Zig helper; not currently wired into a
/// command (the OCEAN web UI embeds the height in the message text).
#[allow(dead_code)]
pub fn validate_block_height(h: &str) -> bool {
    if h == "latest" {
        return true;
    }
    !h.is_empty() && h.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_garbage() {
        assert!(validate_bolt12_offer("lnbc1234").is_err());
        assert!(validate_bolt12_offer("").is_err());
        assert!(validate_bolt12_offer("lno").is_err());
        assert!(validate_bolt12_offer("not-an-offer").is_err());
    }

    #[test]
    fn block_height() {
        assert!(validate_block_height("latest"));
        assert!(validate_block_height("840000"));
        assert!(!validate_block_height(""));
        assert!(!validate_block_height("abc"));
        assert!(!validate_block_height("840 000"));
    }
}
