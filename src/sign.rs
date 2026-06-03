//! BIP-322 message signing from a BIP39 mnemonic.
//!
//! Software signing via the `bip322` crate (replacing the project's
//! earlier hardware-wallet flow). Output is a base64-encoded witness,
//! exactly what the OCEAN web interface expects.

use crate::error::{Error, Result};
use bip322::sign_simple_encoded;
use bip39::Mnemonic;
use bitcoin::bip32::{ChildNumber, DerivationPath, Xpriv};
use bitcoin::secp256k1::Secp256k1;
use bitcoin::{Address, AddressType, CompressedPublicKey, Network, PrivateKey};
use std::io::IsTerminal;
use std::str::FromStr;
use zeroize::{Zeroize, Zeroizing};

/// Default BIP84 P2WPKH path: `m/84'/0'/0'/0/0`.
pub const DEFAULT_BIP32_PATH: &str = "m/84'/0'/0'/0/0";

/// Heap-allocated mnemonic wrapper that is zeroed on drop.
pub struct MnemonicSecret(String);

impl MnemonicSecret {
    pub(crate) fn new(s: String) -> Self {
        Self(s)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Drop for MnemonicSecret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Prompt for a BIP39 mnemonic on stdin with terminal echo disabled.
///
/// If stdin is not a TTY (e.g. test harness, shell pipe) reads a line
/// of plaintext instead — required for `tests/integration.rs`.
pub fn prompt_mnemonic() -> Result<MnemonicSecret> {
    let raw = if std::io::stdin().is_terminal() {
        rpassword::prompt_password("BIP39 mnemonic (24 words, hidden): ")?
    } else {
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf)?;
        buf
    };
    Ok(MnemonicSecret::new(normalize_whitespace(&raw)))
}

fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse + validate a 24-word BIP39 mnemonic.
///
/// We enforce exactly 24 words to match Lexe's `RootSeed` contract
/// (256 bits of entropy → 24 words). Other word counts parse via BIP39
/// but are not supported here — fail loud and early.
pub fn parse_mnemonic(secret: &MnemonicSecret) -> Result<Mnemonic> {
    let word_count = secret.as_str().split_whitespace().count();
    if word_count != 24 {
        return Err(Error::InvalidMnemonic(format!(
            "expected 24 words, got {word_count}"
        )));
    }
    Mnemonic::parse(secret.as_str()).map_err(|e| Error::InvalidMnemonic(format!("{e}")))
}

/// Generate a fresh 24-word BIP39 mnemonic (256 bits of entropy).
///
/// 24 words to match the count [`parse_mnemonic`] enforces, so the same
/// seed is usable both here (BIP-322 signing) and as the Lexe sidecar's
/// `RootSeed` (`LEXE_ROOT_SEED` / `LEXE_ROOT_SEED_PATH` accept a
/// mnemonic). Entropy comes from the OS CSPRNG via `bip39`'s `rand`
/// feature; the intermediate `Mnemonic` is `ZeroizeOnDrop` and the
/// returned [`MnemonicSecret`] wipes its heap string on drop. The caller
/// is responsible for displaying it exactly once.
pub fn generate_mnemonic() -> Result<MnemonicSecret> {
    let m = Mnemonic::generate(24).map_err(|e| Error::InvalidMnemonic(format!("{e}")))?;
    Ok(MnemonicSecret::new(m.to_string()))
}

/// Derive a BIP32 child private key from a mnemonic at the given path.
///
/// Uses the standard BIP39 PBKDF2 seed (empty passphrase) →
/// `Xpriv::new_master(Network::Bitcoin, …)` → `derive_priv`. The PBKDF2
/// seed buffer is wrapped in `Zeroizing` so the stack copy is wiped on
/// drop rather than left as residue.
pub fn derive_private_key(m: &Mnemonic, path: &DerivationPath) -> Result<PrivateKey> {
    let seed: Zeroizing<[u8; 64]> = Zeroizing::new(m.to_seed(""));
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(Network::Bitcoin, seed.as_ref())
        .map_err(|e| Error::SigningFailed(format!("xpriv master: {e}")))?;
    let child = master
        .derive_priv(&secp, path)
        .map_err(|e| Error::SigningFailed(format!("derive: {e}")))?;
    Ok(PrivateKey::new(child.private_key, Network::Bitcoin))
}

/// P2WPKH mainnet address (`bc1q…`) for an already-derived key.
pub fn address_from_key(key: &PrivateKey) -> Result<String> {
    let secp = Secp256k1::new();
    let cpk = CompressedPublicKey::from_private_key(&secp, key)
        .map_err(|e| Error::SigningFailed(format!("compressed pubkey: {e}")))?;
    Ok(Address::p2wpkh(&cpk, Network::Bitcoin).to_string())
}

/// Derive the BIP84 P2WPKH mainnet address (`bc1q…`) for a mnemonic at a path.
///
/// This is the address the user registers with OCEAN as their mining payout
/// destination. Deriving it from the same mnemonic we sign with guarantees the
/// key controls the address — callers never have to supply (and can't mistype)
/// an address that the key doesn't own. When you already hold the derived key,
/// call [`address_from_key`] to avoid re-running PBKDF2.
pub fn derive_address(m: &Mnemonic, path: &DerivationPath) -> Result<String> {
    address_from_key(&derive_private_key(m, path)?)
}

/// Parse a BIP32 path string like `m/84'/0'/0'/0/0`.
///
/// Accepts both `'` and `h` for hardened-index markers, an optional
/// leading `m/`, and rejects empty paths.
pub fn parse_bip32_path(s: &str) -> Result<DerivationPath> {
    let trimmed = s
        .strip_prefix("m/")
        .or_else(|| s.strip_prefix('m'))
        .unwrap_or(s);

    let components: std::result::Result<Vec<ChildNumber>, Error> = trimmed
        .split('/')
        .filter(|p| !p.is_empty())
        .map(|p| -> Result<ChildNumber> {
            let (num_str, hardened) = if let Some(stripped) = p.strip_suffix('\'') {
                (stripped, true)
            } else if let Some(stripped) = p.strip_suffix('h') {
                (stripped, true)
            } else {
                (p, false)
            };
            let num: u32 = num_str
                .parse()
                .map_err(|_| Error::InvalidBip32Path(s.to_string()))?;
            if hardened {
                ChildNumber::from_hardened_idx(num)
                    .map_err(|_| Error::InvalidBip32Path(s.to_string()))
            } else {
                ChildNumber::from_normal_idx(num)
                    .map_err(|_| Error::InvalidBip32Path(s.to_string()))
            }
        })
        .collect();

    let components = components?;
    if components.is_empty() {
        return Err(Error::InvalidBip32Path(s.to_string()));
    }
    Ok(DerivationPath::from(components))
}

/// Validate that an address is a P2WPKH mainnet address (`bc1q...`).
fn require_p2wpkh_mainnet(address: &str) -> Result<()> {
    let parsed = Address::from_str(address)
        .map_err(|_| Error::AddressNotP2wpkh(address.to_string()))?
        .require_network(Network::Bitcoin)
        .map_err(|_| Error::AddressNotP2wpkh(address.to_string()))?;

    if parsed.address_type() != Some(AddressType::P2wpkh) {
        return Err(Error::AddressNotP2wpkh(address.to_string()));
    }
    Ok(())
}

/// Sign a message with BIP-322 simple mode using an already-derived key.
///
/// Returns a base64-encoded witness — the exact format the OCEAN web
/// interface expects. Takes the derived key rather than the mnemonic so the
/// caller can derive once and reuse it for the address too.
pub fn sign_bip322(key: &PrivateKey, address: &str, message: &str) -> Result<String> {
    require_p2wpkh_mainnet(address)?;
    // `WIF` is a base58check serialization of the secret key. Wrap in
    // `Zeroizing` so the heap bytes are wiped once signing returns.
    let wif: Zeroizing<String> = Zeroizing::new(key.to_wif());
    sign_simple_encoded(address, message, wif.as_str())
        .map_err(|e| Error::SigningFailed(format!("{e:?}")))
}

// ── tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MNEMONIC: &str = "music mystery deliver gospel profit blanket leaf tell photo segment letter degree nice plastic duty canyon mammal marble bicycle economy unique find cream dune";

    #[test]
    fn parses_default_path() {
        let p = parse_bip32_path(DEFAULT_BIP32_PATH).unwrap();
        let v: Vec<ChildNumber> = p.into_iter().copied().collect();
        assert_eq!(v.len(), 5);
        assert_eq!(v[0], ChildNumber::from_hardened_idx(84).unwrap());
        assert_eq!(v[1], ChildNumber::from_hardened_idx(0).unwrap());
        assert_eq!(v[2], ChildNumber::from_hardened_idx(0).unwrap());
        assert_eq!(v[3], ChildNumber::from_normal_idx(0).unwrap());
        assert_eq!(v[4], ChildNumber::from_normal_idx(0).unwrap());
    }

    #[test]
    fn parses_h_notation() {
        let p = parse_bip32_path("m/49h/0h/0h/0/1").unwrap();
        let v: Vec<ChildNumber> = p.into_iter().copied().collect();
        assert_eq!(v[0], ChildNumber::from_hardened_idx(49).unwrap());
        assert_eq!(v[4], ChildNumber::from_normal_idx(1).unwrap());
    }

    #[test]
    fn parses_no_m_prefix() {
        let p = parse_bip32_path("84'/0'/0'/0/0").unwrap();
        let v: Vec<ChildNumber> = p.into_iter().copied().collect();
        assert_eq!(v.len(), 5);
    }

    #[test]
    fn rejects_garbage_path() {
        assert!(parse_bip32_path("not-a-path").is_err());
        assert!(parse_bip32_path("").is_err());
        assert!(parse_bip32_path("m/").is_err());
    }

    #[test]
    fn requires_24_word_mnemonic() {
        let twelve = MnemonicSecret::new(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".into()
        );
        match parse_mnemonic(&twelve) {
            Err(Error::InvalidMnemonic(msg)) => assert!(msg.contains("24 words")),
            other => panic!("expected InvalidMnemonic, got {other:?}"),
        }
    }

    #[test]
    fn rejects_non_p2wpkh_address() {
        let m = Mnemonic::parse(TEST_MNEMONIC).unwrap();
        let path = parse_bip32_path(DEFAULT_BIP32_PATH).unwrap();
        let key = derive_private_key(&m, &path).unwrap();
        // P2TR (taproot) — must be rejected
        let r = sign_bip322(
            &key,
            "bc1ppv609nr0vr25u07u95waq5lucwfm6tde4nydujnu8npg4q75mr5sxq8lt3",
            "hello",
        );
        assert!(matches!(r, Err(Error::AddressNotP2wpkh(_))));
    }

    #[test]
    fn generates_valid_24_word_mnemonic() {
        let secret = generate_mnemonic().unwrap();
        assert_eq!(secret.as_str().split_whitespace().count(), 24);
        // Must pass the same validation real users hit, and derive a key.
        let m = parse_mnemonic(&secret).unwrap();
        let path = parse_bip32_path(DEFAULT_BIP32_PATH).unwrap();
        derive_private_key(&m, &path).unwrap();
    }

    #[test]
    fn generated_mnemonics_are_unique() {
        let a = generate_mnemonic().unwrap();
        let b = generate_mnemonic().unwrap();
        assert_ne!(a.as_str(), b.as_str());
    }

    #[test]
    fn derives_p2wpkh_address_that_key_controls() {
        let m = Mnemonic::parse(TEST_MNEMONIC).unwrap();
        let path = parse_bip32_path(DEFAULT_BIP32_PATH).unwrap();
        let addr = derive_address(&m, &path).unwrap();
        // Native segwit v0 mainnet address.
        assert!(addr.starts_with("bc1q"), "got {addr}");
        // Deterministic.
        assert_eq!(addr, derive_address(&m, &path).unwrap());
        // The derived address is, by construction, P2WPKH-mainnet — so signing
        // against it must pass the address guard and succeed.
        let key = derive_private_key(&m, &path).unwrap();
        let sig = sign_bip322(&key, &addr, "hello").unwrap();
        assert!(!sig.is_empty());
    }

    #[test]
    fn derives_deterministic_key() {
        let m = Mnemonic::parse(TEST_MNEMONIC).unwrap();
        let path = parse_bip32_path(DEFAULT_BIP32_PATH).unwrap();
        let k1 = derive_private_key(&m, &path).unwrap();
        let k2 = derive_private_key(&m, &path).unwrap();
        assert_eq!(k1.to_wif(), k2.to_wif());
    }
}
