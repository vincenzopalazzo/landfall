use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not reach sidecar at {url} — is `lexe-sidecar` running? (cause: {source})")]
    SidecarUnreachable { url: String, source: reqwest::Error },

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API ({code}): {msg}")]
    Api { code: u16, msg: String },

    #[error("invalid BOLT12 offer: {0}")]
    InvalidOffer(String),

    #[error("invalid mnemonic: {0}")]
    InvalidMnemonic(String),

    #[error("only P2WPKH (bc1q...) addresses are supported: {0}")]
    AddressNotP2wpkh(String),

    #[error("invalid BIP32 path '{0}'")]
    InvalidBip32Path(String),

    #[error("seed file {path} already exists with a different seed; pass force to overwrite")]
    SeedExists { path: String },

    #[error("BIP-322 signing failed: {0}")]
    SigningFailed(String),

    #[error("lexe wallet: {0}")]
    Wallet(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
