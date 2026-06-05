//! OCEAN Lightning desktop shell (Tauri v2).
//!
//! The webview runs the existing `oceanln-web` wizard, but instead of talking to
//! a loopback `oceanln-httpd` server it reaches these `#[tauri::command]`s over
//! native IPC. There is no HTTP server, no port, and no bearer token in the
//! desktop build — the smallest possible attack surface. Each command is a thin
//! adapter over `oceanln_httpd::service`, so the signing/seed security logic is
//! the exact same audited code path the HTTP transport uses.

use std::sync::Arc;

use serde::Serialize;
use tauri::Manager;

use oceanln_common::error::Error;
use oceanln_common::seed::SeedSource;
use oceanln_common::sign::DEFAULT_BIP32_PATH;
use oceanln_httpd::{service, LexeWalletProvider, WalletProvider};

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
fn generate(state: tauri::State<'_, DesktopState>) -> Result<service::GenerateResp, CommandError> {
    service::generate(&state.seed, &state.default_path).map_err(Into::into)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            // creates it 0600 on first generate/import.
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(DesktopState {
                seed: SeedSource::File(dir.join("seed")),
                default_path: DEFAULT_BIP32_PATH.to_string(),
                wallet: Arc::new(LexeWalletProvider),
                sidecar_url: oceanln_common::client::DEFAULT_BASE_URL.to_string(),
                sidecar_credentials: None,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            generate,
            import_seed,
            create_offer,
            init_wallet,
            payout
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
