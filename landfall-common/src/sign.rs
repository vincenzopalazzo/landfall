//! BIP-322 message signing from a BIP39 mnemonic.
//!
//! Software signing via the `bip322` crate (replacing the project's
//! earlier hardware-wallet flow). Output is a base64-encoded witness,
//! exactly what the OCEAN web interface expects.

use crate::error::{Error, Result};
use bip322::{sign_simple_encoded, verify_simple_encoded, Verification, SIMPLE_SIGNATURE_PREFIX};
use bip39::Mnemonic;
use bitcoin::bip32::{ChildNumber, DerivationPath, Xpriv};
use bitcoin::secp256k1::Secp256k1;
use bitcoin::{Address, AddressType, CompressedPublicKey, Network, PrivateKey};
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
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

    /// Build a secret from caller-supplied input (e.g. an imported phrase),
    /// collapsing every run of whitespace to a single space. The caller is
    /// responsible for wiping the original buffer if it outlives this call.
    pub fn from_input(raw: &str) -> Self {
        Self(normalize_whitespace(raw))
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

/// Resolve a BIP39 mnemonic without forcing the user to re-type it every run.
///
/// Precedence, highest first:
///   1. `seed_file` — an explicit `--seed-file <path>` override
///   2. piped stdin (non-TTY) — used by the test harness and shell pipes
///   3. the persisted managed seed file ([`default_seed_path`]), if present
///   4. an interactive hidden prompt (TTY only)
///
/// An empty pipe at step 2 falls through to the managed file rather than
/// erroring, so a closed stdin never masks a stored seed. Returns an error
/// when none apply (e.g. non-interactive with no seed source).
pub fn resolve_seed(seed_file: Option<&Path>) -> Result<MnemonicSecret> {
    // 1. Explicit --seed-file always wins.
    if let Some(path) = seed_file {
        return read_seed_file(path);
    }

    // 2. Piped stdin (non-TTY): test harness, shell pipes.
    if !std::io::stdin().is_terminal() {
        // Read the WHOLE pipe, not one line: `cat seedfile | landfall …` with
        // one word per line is as valid as a single-line phrase (QA-011).
        let mut buf = Zeroizing::new(String::new());
        std::io::stdin().read_to_string(&mut buf)?;
        let secret = MnemonicSecret::new(normalize_whitespace(&buf));
        if !secret.as_str().is_empty() {
            return Ok(secret);
        }
        // Empty pipe — fall through to the managed file.
    }

    // 3. The persisted managed seed file.
    if let Some(path) = default_seed_path() {
        if path.exists() {
            return read_seed_file(&path);
        }
    }

    // 4. Interactive hidden prompt — only possible on a TTY.
    if std::io::stdin().is_terminal() {
        let raw = Zeroizing::new(rpassword::prompt_password(
            "BIP39 mnemonic (24 words, hidden): ",
        )?);
        return Ok(MnemonicSecret::new(normalize_whitespace(&raw)));
    }

    Err(Error::InvalidMnemonic(
        "no seed available: stdin is empty, no seed file found, and no TTY to prompt on \
         (run `landfall init` first, or pass --seed-file)"
            .into(),
    ))
}

/// Name of the managed config directory.
const CONFIG_DIR_NAME: &str = "landfall";
/// The project's name before it became Landfall. A wallet persisted under
/// this directory keeps working (see [`seed_path_preferring_existing`]).
pub const LEGACY_CONFIG_DIR_NAME: &str = "oceanln";

/// Default location of the persisted seed file: `$XDG_CONFIG_HOME/landfall/seed`,
/// falling back to `~/.config/landfall/seed`. If only the pre-rename
/// `…/oceanln/seed` exists, that file is used instead, for reads and writes.
/// `None` if neither `XDG_CONFIG_HOME` nor `HOME` is set.
pub fn default_seed_path() -> Option<PathBuf> {
    config_base().map(|base| {
        seed_path_preferring_existing(
            &base.join(CONFIG_DIR_NAME),
            &base.join(LEGACY_CONFIG_DIR_NAME),
        )
    })
}

/// Pick the seed file when a renamed location may still hold a wallet from
/// before the rename: `new_dir/seed`, unless only `legacy_dir/seed` exists.
/// Then the legacy file keeps being used for reads AND writes, so an upgrade
/// never makes a stored wallet look missing, and `init` never creates a
/// second, different seed beside it. Shared by the CLI/httpd managed path and
/// the desktop app's data dir.
pub fn seed_path_preferring_existing(new_dir: &Path, legacy_dir: &Path) -> PathBuf {
    let new = new_dir.join("seed");
    let legacy = legacy_dir.join("seed");
    if !new.exists() && legacy.exists() {
        legacy
    } else {
        new
    }
}

/// The XDG config base: `$XDG_CONFIG_HOME`, else `~/.config`.
fn config_base() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg));
        }
    }
    let home = std::env::var("HOME").ok().filter(|h| !h.is_empty())?;
    Some(PathBuf::from(home).join(".config"))
}

/// Read a mnemonic from a seed file, enforcing owner-only perms on unix.
///
/// A group/world-accessible seed file is a hard error — we refuse to read a
/// mainnet seed that other local users could too.
fn read_seed_file(path: &Path) -> Result<MnemonicSecret> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(path)
            .map_err(|e| Error::Wallet(format!("cannot read seed file {}: {e}", path.display())))?;
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(Error::Wallet(format!(
                "seed file {} has insecure permissions {mode:04o}; \
                 run `chmod 600 {}` (must not be group/world-accessible)",
                path.display(),
                path.display(),
            )));
        }
    }
    let raw =
        Zeroizing::new(std::fs::read_to_string(path).map_err(|e| {
            Error::Wallet(format!("cannot read seed file {}: {e}", path.display()))
        })?);
    let secret = MnemonicSecret::new(normalize_whitespace(&raw));
    if secret.as_str().is_empty() {
        return Err(Error::Wallet(format!(
            "seed file {} is empty",
            path.display()
        )));
    }
    Ok(secret)
}

/// Persist a mnemonic to the managed seed file (or `dest` override) so future
/// `offer` / `payout` runs derive keys without re-prompting.
///
/// Writes `0600` on unix and creates the parent dir `0700`. Refuses to clobber
/// an existing file holding a *different* seed unless `force`; an identical
/// existing file is a no-op, so re-running `init` is idempotent. Returns the
/// path written.
#[cfg(feature = "lexe-sdk")]
pub fn store_seed(secret: &MnemonicSecret, dest: Option<&Path>, force: bool) -> Result<PathBuf> {
    let path = match dest {
        Some(p) => p.to_path_buf(),
        None => default_seed_path().ok_or_else(|| {
            Error::Wallet(
                "cannot locate a config dir (set HOME or XDG_CONFIG_HOME, or pass --seed-file)"
                    .into(),
            )
        })?,
    };

    let existed = path.exists();
    if existed {
        let existing = Zeroizing::new(std::fs::read_to_string(&path).map_err(|e| {
            Error::Wallet(format!(
                "cannot read existing seed file {}: {e}",
                path.display()
            ))
        })?);
        if normalize_whitespace(&existing) == secret.as_str() {
            // Identical seed already present: no rewrite needed, but still
            // re-assert 0600 so a manually-created loose file becomes usable by
            // later `offer` / `payout` (which reject group/world-readable seeds).
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).map_err(
                    |e| Error::Wallet(format!("cannot set perms on {}: {e}", path.display())),
                )?;
            }
            return Ok(path); // idempotent: identical contents need no --force
        }
        if !force {
            return Err(Error::SeedExists {
                path: path.display().to_string(),
            });
        }
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Wallet(format!("cannot create {}: {e}", parent.display())))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
            }
        }
    }

    // Fresh create → exclusive (atomic `O_EXCL`): if another writer created the
    // seed file after our `exists()` check above (two `/generate` or `/import`
    // requests racing on a new wallet), the open fails with `SeedExists` rather
    // than clobbering it, so two clients can never persist diverging seeds and
    // back up a phrase that doesn't match disk. Force-overwrite truncates.
    write_secret_file(&path, secret.as_str(), !existed)?;
    Ok(path)
}

/// Write `contents` to `path` as a `0600` file.
///
/// `exclusive` selects atomic create-new (`O_EXCL`, fails if the file already
/// exists) for the fresh-wallet path; otherwise truncates an existing file
/// (the explicit `--force`/`force:true` overwrite).
#[cfg(feature = "lexe-sdk")]
fn write_secret_file(path: &Path, contents: &str, exclusive: bool) -> Result<()> {
    use std::io::Write;
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true);
    if exclusive {
        opts.create_new(true);
    } else {
        opts.create(true).truncate(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600); // applies only when the file is freshly created
    }
    let mut f = match opts.open(path) {
        Ok(f) => f,
        Err(e) if exclusive && e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Lost a create race: another writer persisted a seed first.
            return Err(Error::SeedExists {
                path: path.display().to_string(),
            });
        }
        Err(e) => {
            return Err(Error::Wallet(format!(
                "cannot write seed file {}: {e}",
                path.display()
            )))
        }
    };
    // Re-assert 0600 so a --force overwrite of a pre-existing (possibly looser)
    // file is also tightened, not just freshly-created ones.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        f.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|e| Error::Wallet(format!("cannot set perms on {}: {e}", path.display())))?;
    }
    // Trailing newline; `normalize_whitespace` strips it on read.
    writeln!(f, "{contents}")
        .map_err(|e| Error::Wallet(format!("cannot write seed file {}: {e}", path.display())))?;
    Ok(())
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

/// Generate a 256-bit random bearer token as a 64-char lowercase hex string.
///
/// Drawn from the same OS CSPRNG `bitcoin`/`secp256k1` already uses, so no new
/// dependency is needed. Used by `landfall-httpd` to mint a loopback bearer
/// token when the operator does not supply one.
pub fn random_token() -> String {
    use bitcoin::secp256k1::rand::RngCore;
    let mut bytes = [0u8; 32];
    bitcoin::secp256k1::rand::thread_rng().fill_bytes(&mut bytes);
    let mut s = String::with_capacity(64);
    for b in bytes {
        // Two lowercase hex digits per byte, no external hex dependency.
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0x0f) as u32, 16).unwrap());
    }
    s
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
///
/// The key must control `address`: a BIP-322 signature only proves
/// ownership of the address it is verified against, so signing for an
/// address the key does not control would yield a witness that no correct
/// verifier accepts. Rather than let that surface later as "signature
/// invalid" on the OCEAN side, refuse here.
pub fn sign_bip322(key: &PrivateKey, address: &str, message: &str) -> Result<String> {
    require_p2wpkh_mainnet(address)?;
    let controlled = address_from_key(key)?;
    if controlled != address {
        return Err(Error::SigningFailed(format!(
            "key controls {controlled}, not {address}; refusing to sign for an address \
             the key cannot prove ownership of"
        )));
    }
    // `WIF` is a base58check serialization of the secret key. Wrap in
    // `Zeroizing` so the heap bytes are wiped once signing returns.
    let wif: Zeroizing<String> = Zeroizing::new(key.to_wif());
    let encoded = sign_simple_encoded(address, message, &[wif.as_str()], None)
        .map_err(|e| Error::SigningFailed(format!("{e:?}")))?;
    // Since 0.0.12 the `bip322` crate tags its output with a variant prefix
    // (`smp` for simple mode). That prefix is the crate's own convention, not
    // part of BIP-322: OCEAN, Bitcoin Core and every other verifier expect
    // the bare base64 witness and reject `smp…` with "signature check
    // failed". Strip it so the wire format stays what it was before the bump.
    Ok(encoded
        .strip_prefix(SIMPLE_SIGNATURE_PREFIX)
        .map(str::to_owned)
        .unwrap_or(encoded))
}

/// Verify a BIP-322 simple-mode signature for a P2WPKH mainnet address.
///
/// This is the check OCEAN runs on submission: the base64 witness must be a
/// valid signature over `message` by the key that controls `address`. It is
/// exposed so callers (the CLI `verify` command, the QA harness, a UI "check
/// before you paste" step) can confirm a signature offline, without the key.
///
/// Returns `Ok(())` on a valid signature and an [`Error::SigningFailed`]
/// describing why otherwise (wrong key, tampered message, malformed
/// witness). A signature the verifier cannot interpret is reported as
/// invalid, never as valid.
pub fn verify_bip322(address: &str, message: &str, signature: &str) -> Result<()> {
    require_p2wpkh_mainnet(address)?;
    match verify_simple_encoded(address, message, signature) {
        Ok(Verification::Valid { .. }) => Ok(()),
        Ok(Verification::Inconclusive) => Err(Error::SigningFailed(
            "signature could not be interpreted for this address".to_string(),
        )),
        Err(e) => Err(Error::SigningFailed(format!("invalid signature: {e}"))),
    }
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
        verify_bip322(&addr, "hello", &sig).unwrap();
    }

    /// BIP-84 test vector (the `abandon`×11 `about` mnemonic): the first
    /// external address at `m/84'/0'/0'/0/0` is pinned in the BIP itself.
    /// Guards the whole derivation chain (PBKDF2 → xpriv → child → P2WPKH),
    /// not just "starts with bc1q".
    #[test]
    fn bip84_vector_first_address() {
        let m = Mnemonic::parse(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        )
        .unwrap();
        let path = parse_bip32_path(DEFAULT_BIP32_PATH).unwrap();
        assert_eq!(
            derive_address(&m, &path).unwrap(),
            "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"
        );
    }

    /// Bitcoin Core's BIP-322 vector (`src/test/util_tests.cpp`): key
    /// `L3VF…` controls `bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l`. Both the
    /// BIP's reference signature and Core's must verify, and our signer must
    /// produce a signature that verifies for the same (address, message).
    const CORE_WIF: &str = "L3VFeEujGtevx9w18HD1fhRbCH67Az2dpCymeRE1SoPK6XQtaN2k";
    const CORE_ADDR: &str = "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l";
    const BIP322_REF_SIG_HELLO_WORLD: &str = "AkcwRAIgZRfIY3p7/DoVTty6YZbWS71bc5Vct9p9Fia83eRmw2QCICK/ENGfwLtptFluMGs2KsqoNSk89pO7F29zJLUx9a/sASECx/EgAxlkQpQ9hYjgGu6EBCPMVPwVIVJqO4XCsMvViHI=";

    #[test]
    fn verifies_bip322_reference_signature() {
        verify_bip322(CORE_ADDR, "Hello World", BIP322_REF_SIG_HELLO_WORLD).unwrap();
        // Tampered message, same signature → must fail.
        assert!(verify_bip322(CORE_ADDR, "Hello Mars", BIP322_REF_SIG_HELLO_WORLD).is_err());
    }

    #[test]
    fn signs_core_vector_key_and_round_trips() {
        let key = PrivateKey::from_wif(CORE_WIF).unwrap();
        assert_eq!(address_from_key(&key).unwrap(), CORE_ADDR);
        let sig = sign_bip322(&key, CORE_ADDR, "Hello World").unwrap();
        verify_bip322(CORE_ADDR, "Hello World", &sig).unwrap();
        assert!(verify_bip322(CORE_ADDR, "Hello Mars", &sig).is_err());
    }

    /// The wire format OCEAN accepts is the bare base64 consensus encoding
    /// of the witness, nothing else. `bip322` 0.0.12 started prefixing its
    /// encoded output with `smp`, and shipping that verbatim made OCEAN
    /// reject every Landfall signature with "signature check failed", while
    /// our own `verify_bip322` (which tolerates the prefix) kept passing.
    /// Pin the format independently of the crate: decode the string as
    /// plain base64 and parse a two-item P2WPKH witness out of it.
    #[test]
    fn signature_is_bare_base64_witness_without_variant_prefix() {
        use bitcoin::consensus::Decodable;

        let key = PrivateKey::from_wif(CORE_WIF).unwrap();
        let sig = sign_bip322(&key, CORE_ADDR, "Hello World").unwrap();

        assert!(
            !sig.starts_with(SIMPLE_SIGNATURE_PREFIX),
            "signature carries the crate's `{SIMPLE_SIGNATURE_PREFIX}` prefix: {sig}"
        );
        // Same shape as the BIP-322 reference signature: a 2-item witness
        // always encodes to a string starting with `Ak` (0x02 count byte).
        assert!(sig.starts_with("Ak"), "unexpected encoding: {sig}");

        // Decode the way a third-party verifier would: standard base64 →
        // consensus witness, no prefix handling at all.
        let bytes = base64_decode_standard(&sig).expect("plain base64");
        let witness = bitcoin::Witness::consensus_decode(&mut bytes.as_slice())
            .expect("consensus-encoded witness");
        assert_eq!(witness.len(), 2, "P2WPKH witness is <sig> <pubkey>");
        assert_eq!(
            witness.last().unwrap(),
            key.public_key(&Secp256k1::new()).to_bytes()
        );
    }

    /// Minimal strict standard-base64 decoder so the test does not depend on
    /// the same crate whose behaviour it pins.
    fn base64_decode_standard(s: &str) -> Option<Vec<u8>> {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let s = s.as_bytes();
        if !s.len().is_multiple_of(4) {
            return None;
        }
        let mut out = Vec::with_capacity(s.len() / 4 * 3);
        for chunk in s.chunks(4) {
            let mut acc: u32 = 0;
            let mut pad = 0;
            for &c in chunk {
                acc <<= 6;
                if c == b'=' {
                    pad += 1;
                } else {
                    acc |= ALPHABET.iter().position(|&a| a == c)? as u32;
                }
            }
            let bytes = acc.to_be_bytes();
            out.extend_from_slice(&bytes[1..4 - pad]);
        }
        Some(out)
    }

    /// The key/address binding: signing for an address the key does not
    /// control must be refused up front, and a signature made by another
    /// key must never verify for this address. (With `bip322` 0.0.10 the
    /// verifier accepted exactly that; this test pins the fix.)
    #[test]
    fn refuses_to_sign_for_address_key_does_not_control() {
        let m = Mnemonic::parse(TEST_MNEMONIC).unwrap();
        let path = parse_bip32_path(DEFAULT_BIP32_PATH).unwrap();
        let key = derive_private_key(&m, &path).unwrap();
        let own = derive_address(&m, &path).unwrap();
        assert_ne!(own, CORE_ADDR);

        let err = sign_bip322(&key, CORE_ADDR, "hello").unwrap_err();
        assert!(matches!(err, Error::SigningFailed(_)), "got {err}");

        // A signature by our key over our address is not a signature for
        // CORE_ADDR, whatever the message.
        let sig = sign_bip322(&key, &own, "hello").unwrap();
        assert!(verify_bip322(CORE_ADDR, "hello", &sig).is_err());
        verify_bip322(&own, "hello", &sig).unwrap();
    }

    #[test]
    fn verify_rejects_garbage_and_non_p2wpkh() {
        assert!(verify_bip322(CORE_ADDR, "x", "not base64!").is_err());
        assert!(verify_bip322(CORE_ADDR, "x", "").is_err());
        // Legacy address → address guard, not a crash.
        assert!(verify_bip322(
            "1BitcoinEaterAddressDontSendf59kuE",
            "x",
            BIP322_REF_SIG_HELLO_WORLD
        )
        .is_err());
    }

    /// The rename to Landfall must not make an existing wallet look missing:
    /// a seed persisted under the old `oceanln` directory keeps being used
    /// until one exists under the new name, and never alongside it.
    #[test]
    fn managed_seed_path_keeps_a_pre_rename_wallet() {
        let base = std::env::temp_dir().join(format!("landfall-legacy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let new_dir = base.join("landfall");
        let legacy_dir = base.join(LEGACY_CONFIG_DIR_NAME);
        let pick = || seed_path_preferring_existing(&new_dir, &legacy_dir);

        // Nothing anywhere: a fresh install writes under the new name.
        assert_eq!(pick(), new_dir.join("seed"));

        // Only the pre-rename seed: keep using it (reads and writes).
        std::fs::create_dir_all(&legacy_dir).unwrap();
        std::fs::write(legacy_dir.join("seed"), "x").unwrap();
        assert_eq!(pick(), legacy_dir.join("seed"));

        // Both exist: the new one wins.
        std::fs::create_dir_all(&new_dir).unwrap();
        std::fs::write(new_dir.join("seed"), "y").unwrap();
        assert_eq!(pick(), new_dir.join("seed"));

        let _ = std::fs::remove_dir_all(&base);
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

// Seed-file persistence tests. Gated to `lexe-sdk` (where `store_seed` lives)
// and `unix` (where the 0600 perms contract applies). Each test uses an
// explicit `--seed-file`-style path, so they never touch HOME/XDG and stay
// order-independent under parallel execution.
#[cfg(all(test, unix, feature = "lexe-sdk"))]
mod seed_file_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    const TEST_MNEMONIC: &str = "music mystery deliver gospel profit blanket leaf tell photo segment letter degree nice plastic duty canyon mammal marble bicycle economy unique find cream dune";
    // Any other valid 24-word mnemonic, for the conflicting-overwrite case.
    const OTHER_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    // Unique temp dir per call site (line!()), no external tempfile dep.
    fn tmp_dir(label: u32) -> PathBuf {
        std::env::temp_dir().join(format!("landfall-seedtest-{}-{label}", std::process::id()))
    }

    #[test]
    fn store_writes_0600_and_resolve_roundtrips() {
        let dir = tmp_dir(line!());
        let path = dir.join("seed");
        let secret = MnemonicSecret::new(TEST_MNEMONIC.into());

        let written = store_seed(&secret, Some(&path), false).unwrap();
        assert_eq!(written, path);

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "seed file must be 0600, got {mode:04o}");

        let got = resolve_seed(Some(&path)).unwrap();
        assert_eq!(got.as_str(), TEST_MNEMONIC);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn store_is_idempotent_but_refuses_conflicting_overwrite() {
        let dir = tmp_dir(line!());
        let path = dir.join("seed");
        let a = MnemonicSecret::new(TEST_MNEMONIC.into());
        store_seed(&a, Some(&path), false).unwrap();

        // Identical contents — no --force required.
        store_seed(&a, Some(&path), false).unwrap();

        // A different seed — refused without --force...
        let b = MnemonicSecret::new(OTHER_MNEMONIC.into());
        assert!(store_seed(&b, Some(&path), false).is_err());
        // ...allowed with it.
        store_seed(&b, Some(&path), true).unwrap();
        assert_eq!(resolve_seed(Some(&path)).unwrap().as_str(), OTHER_MNEMONIC);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn idempotent_identical_write_tightens_loose_perms_to_0600() {
        let dir = tmp_dir(line!());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("seed");
        // A manually-created, correct-but-loose seed file.
        std::fs::write(&path, TEST_MNEMONIC).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        // Same seed, no --force: the idempotent path must still tighten perms,
        // otherwise `read_seed_file` would reject this very file afterwards.
        let secret = MnemonicSecret::new(TEST_MNEMONIC.into());
        store_seed(&secret, Some(&path), false).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "idempotent store must tighten to 0600, got {mode:04o}"
        );
        // And the file is now actually readable by resolve_seed.
        assert_eq!(resolve_seed(Some(&path)).unwrap().as_str(), TEST_MNEMONIC);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn force_overwrite_tightens_loose_perms_to_0600() {
        let dir = tmp_dir(line!());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("seed");
        std::fs::write(&path, "stale").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        let secret = MnemonicSecret::new(TEST_MNEMONIC.into());
        store_seed(&secret, Some(&path), true).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "overwrite must tighten to 0600, got {mode:04o}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_rejects_group_or_world_readable_seed() {
        let dir = tmp_dir(line!());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("seed");
        std::fs::write(&path, TEST_MNEMONIC).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        match resolve_seed(Some(&path)) {
            Err(Error::Wallet(msg)) => assert!(msg.contains("insecure permissions")),
            Err(other) => panic!("expected insecure-perms Wallet error, got {other:?}"),
            Ok(_) => panic!("expected insecure-perms error, got Ok"),
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_rejects_empty_seed_file() {
        let dir = tmp_dir(line!());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("seed");
        std::fs::write(&path, "").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        match resolve_seed(Some(&path)) {
            Err(Error::Wallet(msg)) => assert!(msg.contains("empty")),
            Err(other) => panic!("expected empty-file Wallet error, got {other:?}"),
            Ok(_) => panic!("expected empty-file error, got Ok"),
        }

        std::fs::remove_dir_all(&dir).ok();
    }
}
