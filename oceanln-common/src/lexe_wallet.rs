//! In-process Lexe wallet operations (feature `lexe-sdk`).
//!
//! Replaces the sidecar HTTP path with direct `lexe` SDK calls, all driven
//! from the user's 24-word seed:
//! - [`init`] creates + provisions the onchain Lexe wallet (`signup`), and
//! - [`create_offer`] mints a payable BOLT12 offer on the node.
//!
//! Mirrors `lexe-cli`'s usage of the same SDK (`init` / `create-offer`).

use std::str::FromStr;

use lexe::config::WalletEnvConfig;
use lexe::types::auth::{CredentialsRef, RootSeed};
use lexe::types::bitcoin::Amount;
use lexe::types::command::CreateOfferRequest;
use lexe::wallet::LexeWallet;

use crate::error::{Error, Result};

/// Parse a 24-word mnemonic into a Lexe `RootSeed`.
fn root_seed(mnemonic: &str) -> Result<RootSeed> {
    let m = lexe::bip39::Mnemonic::from_str(mnemonic.trim())
        .map_err(|e| Error::Wallet(format!("invalid mnemonic: {e}")))?;
    RootSeed::from_mnemonic(m).map_err(|e| Error::Wallet(format!("root seed: {e:#}")))
}

/// Build a stateless (`without_db`) mainnet wallet from the seed.
fn wallet(seed: &RootSeed) -> Result<LexeWallet> {
    LexeWallet::without_db(WalletEnvConfig::mainnet(), CredentialsRef::from(seed))
        .map_err(|e| Error::Wallet(format!("wallet init: {e:#}")))
}

/// Create + provision the onchain Lexe wallet for this seed.
///
/// Equivalent to `lexe init`: registers the user with Lexe's backend and runs
/// the initial provision. Idempotent — safe to call when already signed up.
pub async fn init(mnemonic: &str) -> Result<()> {
    let seed = root_seed(mnemonic)?;
    let wallet = wallet(&seed)?;
    wallet
        .signup(&seed, None)
        .await
        .map_err(|e| Error::Wallet(format!("signup/provision: {e:#}")))
}

/// Create a payable BOLT12 offer on the node; returns the `lno1…` string.
///
/// Requires the wallet to have been provisioned first (see [`init`]).
pub async fn create_offer(
    mnemonic: &str,
    description: Option<&str>,
    min_amount: Option<&str>,
) -> Result<String> {
    let seed = root_seed(mnemonic)?;
    let wallet = wallet(&seed)?;

    let min_amount = match min_amount {
        Some(s) => Some(
            Amount::from_str(s).map_err(|e| Error::Wallet(format!("invalid --min-amount: {e}")))?,
        ),
        None => None,
    };

    let resp = wallet
        .create_offer(CreateOfferRequest {
            description: description.map(String::from),
            min_amount,
            expiration_secs: None,
        })
        .await
        .map_err(|e| Error::Wallet(format!("create offer: {e:#}")))?;

    Ok(resp.offer.to_string())
}
