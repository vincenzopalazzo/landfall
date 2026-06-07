//! Wallet seam — abstracts the in-process Lexe SDK from transport layers.
//!
//! Lives in `oceanln-common` (not `oceanln-httpd`) so transport-binding
//! crates (oceanln-httpd's REST routes, oceanln-mcp's MCP tools, and any
//! future transport) can share the same trait without forming a
//! dependency cycle through oceanln-httpd.
//!
//! The whole module is gated on `lexe-sdk` because the trait references
//! [`crate::lexe_wallet::OceanPayout`] in its method signatures —
//! gating individual methods breaks the `dyn` compatibility the
//! `#[async_trait]` macro relies on (each cfg-gated method changes the
//! v-table shape).

use crate::error::Result;
use crate::lexe_wallet::OceanPayout;

/// The wallet operations every transport exposes, abstracted so call
/// sites don't depend directly on the in-process Lexe SDK. The real
/// implementation is [`LexeWalletProvider`]; tests substitute their own.
#[async_trait::async_trait]
pub trait WalletProvider: Send + Sync {
    /// Provision the onchain wallet for `mnemonic` (idempotent).
    async fn provision(&self, mnemonic: &str) -> Result<()>;

    /// Create a payable BOLT12 offer for `mnemonic`; returns the `lno1…` string.
    async fn create_offer(
        &self,
        mnemonic: &str,
        description: Option<&str>,
        min_amount: Option<&str>,
    ) -> Result<String>;

    /// List the wallet's inbound, completed BOLT12 offer payments — i.e.
    /// OCEAN's Lightning payouts. Empty list = no payouts yet (the Lexe
    /// node call may still have succeeded). `limit` caps the round-trip.
    async fn list_offer_payouts(&self, mnemonic: &str, limit: u16) -> Result<Vec<OceanPayout>>;
}

/// Production [`WalletProvider`] backed by the in-process Lexe SDK
/// ([`crate::lexe_wallet`]).
pub struct LexeWalletProvider;

#[async_trait::async_trait]
impl WalletProvider for LexeWalletProvider {
    async fn provision(&self, mnemonic: &str) -> Result<()> {
        crate::lexe_wallet::init(mnemonic).await
    }

    async fn create_offer(
        &self,
        mnemonic: &str,
        description: Option<&str>,
        min_amount: Option<&str>,
    ) -> Result<String> {
        crate::lexe_wallet::create_offer(mnemonic, description, min_amount).await
    }

    async fn list_offer_payouts(&self, mnemonic: &str, limit: u16) -> Result<Vec<OceanPayout>> {
        crate::lexe_wallet::list_offer_payouts(mnemonic, limit).await
    }
}
