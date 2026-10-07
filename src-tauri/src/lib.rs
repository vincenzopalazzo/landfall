//! Landfall desktop shell (Tauri v2).
//!
//! The webview runs the existing `landfall-web` wizard, but instead of talking to
//! a loopback `landfall-httpd` server it reaches these `#[tauri::command]`s over
//! native IPC. There is no HTTP server, no port, and no bearer token in the
//! desktop build — the smallest possible attack surface. Each command is a thin
//! adapter over `landfall_httpd::service`, so the signing/seed security logic is
//! the exact same audited code path the HTTP transport uses.

use std::sync::Arc;

use serde::Serialize;
use tauri::Manager;

use landfall_common::error::Error;
use landfall_common::seed::SeedSource;
use landfall_common::sign::DEFAULT_BIP32_PATH;
use landfall_httpd::{service, LexeWalletProvider, WalletProvider};

/// Error handed back to the webview. Mirrors the HTTP transport: a status code
/// (so the frontend's `ApiError` logic — e.g. `409` → "wallet exists" — behaves
/// identically on desktop) plus a human message. Never carries the seed.
#[derive(Serialize)]
struct CommandError {
    status: u16,
    message: String,
}

impl From<Error> for CommandError {
    fn from(e: Error) -> Self {
        CommandError {
            status: service::http_status(&e),
            message: e.to_string(),
        }
    }
}

/// List inbound, completed BOLT12 offer payments from the user's Lexe wallet.
///
/// This is the trust-minimized source for OCEAN payouts: we read directly
/// from the user's own node (no ocean.xyz API, no HTML scrape). The OCEAN
/// Lightning payouts arrive at our BOLT12 offer with the block-height /
/// block-hash carried in the BOLT12 payer-supplied `message` field
/// (surfaced as `payer_note` on the wrapper).
#[tauri::command]
async fn list_lightning_payouts(
    state: tauri::State<'_, DesktopState>,
    limit: Option<u16>,
) -> Result<Vec<landfall_common::lexe_wallet::OceanPayout>, CommandError> {
    // Load the mnemonic from the locally-stored seed file. Stays in memory
    // only for the duration of this call (MnemonicSecret zeroizes on drop).
    let secret = state.seed.load()?;
    // Honor the caller-supplied limit (StatsGrid asks for 10_000 so the
    // lifetime aggregate covers every Lightning payout, not just one
    // page). The Rust wallet layer caps the actual scan at
    // `MAX_PAYMENTS_SCANNED = 10_000`, so unbounded requests still
    // terminate. The 200 fallback covers callers that omit the arg.
    let cap = limit.unwrap_or(200);
    state
        .wallet
        .list_offer_payouts(secret.as_str(), cap)
        .await
        .map_err(Into::into)
}

/// In-process backend state. No HTTP server, port, or token — the seed is read
/// locally from the OS app-data dir per request.
struct DesktopState {
    seed: SeedSource,
    default_path: String,
    wallet: Arc<dyn WalletProvider>,
    sidecar_url: String,
    sidecar_credentials: Option<String>,
}

#[tauri::command]
fn wallet_status(state: tauri::State<'_, DesktopState>) -> Result<service::StatusResp, CommandError> {
    service::status(&state.seed, &state.default_path).map_err(Into::into)
}

#[tauri::command]
fn generate(state: tauri::State<'_, DesktopState>) -> Result<service::GenerateResp, CommandError> {
    service::generate(&state.seed, &state.default_path).map_err(Into::into)
}

/// Re-reveal the stored recovery phrase for an explicit, user-initiated
/// backup view (Profile → "Reveal"). Desktop IPC carries no bearer token —
/// the OS user owns both the webview and the seed file, so possession of the
/// session is the auth boundary here, same as `pay`.
#[tauri::command]
fn reveal_seed(state: tauri::State<'_, DesktopState>) -> Result<service::RevealResp, CommandError> {
    service::reveal(&state.seed).map_err(Into::into)
}

#[tauri::command]
fn import_seed(
    state: tauri::State<'_, DesktopState>,
    mnemonic: String,
    force: bool,
) -> Result<service::ImportResp, CommandError> {
    let mut mnemonic = mnemonic;
    service::import(&state.seed, &state.default_path, &mut mnemonic, force).map_err(Into::into)
}

// `tauri::State<'_, T>` is a `Copy` reference wrapper that is `Send` when `T:
// Sync` (DesktopState is), so the borrows below are fine to hold across the
// await — no cloning needed.
#[tauri::command]
async fn create_offer(
    state: tauri::State<'_, DesktopState>,
    description: Option<String>,
    min_amount: Option<String>,
) -> Result<service::OfferResp, CommandError> {
    service::create_offer(
        &state.seed,
        state.wallet.as_ref(),
        description.as_deref(),
        min_amount.as_deref(),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
async fn init_wallet(
    state: tauri::State<'_, DesktopState>,
    path: Option<String>,
) -> Result<service::InitResp, CommandError> {
    service::init(
        &state.seed,
        &state.default_path,
        state.wallet.as_ref(),
        path.as_deref(),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
async fn payout(
    state: tauri::State<'_, DesktopState>,
    message: String,
    offer: Option<String>,
) -> Result<service::PayoutResp, CommandError> {
    service::payout(
        &state.seed,
        &state.default_path,
        &state.sidecar_url,
        state.sidecar_credentials.as_deref(),
        message,
        offer,
        None,
        None,
        None,
    )
    .await
    .map_err(Into::into)
}

/// Live node status + balances (Node wallet cards / "Node online" chip).
#[tauri::command]
async fn node_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<landfall_common::lexe_wallet::NodeStatus, CommandError> {
    service::node_status(&state.seed, state.wallet.as_ref())
        .await
        .map_err(Into::into)
}

/// The node's full payment activity (inbound + outbound, LN + on-chain).
#[tauri::command]
async fn list_payments(
    state: tauri::State<'_, DesktopState>,
    limit: Option<u16>,
) -> Result<Vec<landfall_common::lexe_wallet::Activity>, CommandError> {
    service::list_payments(&state.seed, state.wallet.as_ref(), limit.unwrap_or(200))
        .await
        .map_err(Into::into)
}

/// Create a BOLT11 invoice to receive a payment (Receive flow).
#[tauri::command]
async fn create_invoice(
    state: tauri::State<'_, DesktopState>,
    amount_sats: Option<u64>,
    description: Option<String>,
) -> Result<serde_json::Value, CommandError> {
    let bolt11 = service::create_invoice(
        &state.seed,
        state.wallet.as_ref(),
        amount_sats,
        description.as_deref(),
    )
    .await?;
    Ok(serde_json::json!({ "invoice": bolt11 }))
}

/// Send a payment to any payable string (Send flow). **Moves real funds.**
#[tauri::command]
async fn pay(
    state: tauri::State<'_, DesktopState>,
    payable: String,
    amount_sats: Option<u64>,
    note: Option<String>,
) -> Result<landfall_common::lexe_wallet::PaySummary, CommandError> {
    service::pay(
        &state.seed,
        state.wallet.as_ref(),
        &payable,
        amount_sats,
        note.as_deref(),
    )
    .await
    .map_err(Into::into)
}

/// Open an external URL in the user's default browser.
///
/// The desktop webview can't follow external links itself: WKWebView (macOS)
/// treats `<a target="_blank">` as inert, and the app CSP forbids navigating
/// away from the bundled SPA. So every `ocean.xyz` / `mempool.space` explorer
/// link routes through here, which hands the URL to the OS (`open` / `xdg-open`).
///
/// Hardened: only `http(s)` URLs are ever forwarded — never a `file://`,
/// custom scheme, or shell argument — so a malformed link can't be coerced
/// into opening an arbitrary local target.
#[tauri::command]
fn open_external(url: String) -> Result<(), CommandError> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(CommandError {
            status: 400,
            message: "refusing to open a non-http(s) URL".to_string(),
        });
    }
    open::that(&url).map_err(|e| CommandError {
        status: 500,
        message: format!("could not open browser: {e}"),
    })
}

/// Bundle identifier the desktop app used before the rename to Landfall.
/// Only read, to find a wallet stored under the old app-data dir.
const LEGACY_IDENTIFIER: &str = "xyz.oceanln.desktop";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Linux: webkit2gtk's DMABUF renderer renders a blank white window on
    // GPU-less / virtualized stacks (VMs, many headless or software-GL
    // setups) — the exact "installed app opens to a blank page" symptom.
    // Disabling it forces the software path, which renders everywhere; this
    // app is a lightweight dashboard, so there's no meaningful perf cost.
    // Must be set before the webview process spawns (i.e. before the builder
    // creates the window), and it's inherited by that child process. We only
    // set a default — a user can still force the accelerated path by
    // exporting WEBKIT_DISABLE_DMABUF_RENDERER=0 themselves.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            // Seed lives in the OS app-data dir, e.g.
            // ~/Library/Application Support/<identifier>/seed. `store_seed`
            // creates it 0600 on first generate/import. The app shipped as
            // `xyz.oceanln.desktop` before the rename to Landfall; a wallet
            // stored under that identifier's dir keeps being used until one
            // exists under the new identifier.
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let legacy_dir = dir
                .parent()
                .map(|p| p.join(LEGACY_IDENTIFIER))
                .unwrap_or_else(|| dir.clone());
            let seed_path = landfall_common::sign::seed_path_preferring_existing(&dir, &legacy_dir);
            app.manage(DesktopState {
                seed: SeedSource::File(seed_path),
                default_path: DEFAULT_BIP32_PATH.to_string(),
                wallet: Arc::new(LexeWalletProvider),
                sidecar_url: landfall_common::client::DEFAULT_BASE_URL.to_string(),
                sidecar_credentials: None,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            wallet_status,
            generate,
            reveal_seed,
            import_seed,
            create_offer,
            init_wallet,
            payout,
            list_lightning_payouts,
            node_status,
            list_payments,
            create_invoice,
            pay,
            open_external,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
