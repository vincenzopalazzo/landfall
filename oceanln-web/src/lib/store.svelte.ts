import { OceanlnClient, ApiError, type Backend } from "./api";
import { TauriClient, isTauri } from "./tauri";
import { DEFAULT_BASE, DEFAULT_TOKEN } from "./config";

export type Surface = "wizard" | "profile" | "dashboard";
export type Mode = "create" | "import";

export const STEPS = [
  { key: "welcome", label: "Welcome" },
  { key: "phrase", label: "Recovery phrase" },
  { key: "confirm", label: "Confirm backup" },
  { key: "wallet", label: "Create wallet" },
  { key: "sign", label: "Sign for OCEAN" },
  { key: "done", label: "Turn on payouts" },
] as const;

// Confirm-quiz positions (0-indexed) for a 24-word phrase.
export const CONFIRM_PICKS = [3, 12, 20];

export interface Offer {
  id: string;
  label: string;
  value: string;
}
export interface Address {
  id: string;
  label: string;
  address: string;
  offerId: string | null;
}

// ── Single shared reactive store ──
export const app = $state({
  // connection
  base: DEFAULT_BASE,
  token: DEFAULT_TOKEN,
  serverUp: false,

  // tweaks
  density: "guided" as "guided" | "concise",
  accent: "blue" as "orange" | "blue", // OCEAN blue is the fixed default

  // navigation
  surface: "wizard" as Surface,
  stepIndex: 0,
  mode: "create" as Mode,
  reuse: false, // using a wallet already configured on the server (no phrase to reveal)
  walletExists: false, // /generate reported an existing seed (recover, don't dead-end)

  // wizard inputs
  importWords: Array(24).fill("") as string[],
  revealed: false,
  backedUp: false,
  answers: {} as Record<number, string>,
  offerDescription: "",

  // real artifacts
  // NOTE: `phrase` holds the generated words in JS memory for the session (for
  // the reveal + Profile). JS strings can't be reliably zeroized — inherent to
  // a browser wizard. `restart()` clears it; a native (Tauri) host could do better.
  phrase: [] as string[], // generated words (create mode), revealed once
  miningAddress: "",
  offer: "",
  signature: "",
  message: "", // the message that was signed (echoed back by /payout)
  oceanMessage: "", // the verification message the user pastes from OCEAN

  // Whether the user has confirmed they handed their details to OCEAN. There is
  // no OCEAN-side verification API to call — OCEAN checks the BIP-322 signature
  // on its own servers — so this is a local "I've submitted" acknowledgement,
  // not a network round-trip we can honestly claim to perform.
  submitted: false,

  // post-setup profile
  profile: null as null | { offers: Offer[]; addresses: Address[] },

  // transient UI
  busy: false,
  error: "",
});

// Pick the transport at runtime: native IPC under the Tauri desktop shell (no
// base URL / token), HTTP to oceanln-httpd in the browser.
export function client(): Backend {
  return isTauri() ? new TauriClient() : new OceanlnClient(app.base.replace(/\/$/, ""), app.token);
}

export const guided = () => app.density === "guided";
export const isImport = () => app.mode === "import";
export const stepKey = () => STEPS[app.stepIndex].key;

// ── gating ──
export function canContinue(): boolean {
  switch (stepKey()) {
    case "phrase":
      return isImport()
        ? app.importWords.every((w) => w.trim().length > 1)
        : app.revealed && app.backedUp;
    case "confirm":
      return CONFIRM_PICKS.every((idx, qi) => app.answers[qi] === app.phrase[idx]);
    case "wallet":
      return !!app.offer && !!app.miningAddress;
    case "sign":
      return !!app.signature;
    default:
      return true;
  }
}

// ── navigation ──
// Imported phrases and reused (already-on-server) wallets skip the create-only
// reveal/confirm steps.
const skipConfirm = () => isImport() || app.reuse;

export function chooseMode(m: Mode) {
  app.mode = m;
  app.error = "";
  app.walletExists = false;
  app.stepIndex = 1;
}
export function goNext() {
  let n = app.stepIndex + 1;
  if (STEPS[n]?.key === "confirm" && skipConfirm()) n += 1;
  app.stepIndex = Math.min(n, STEPS.length - 1);
}
export function goBack() {
  let n = app.stepIndex - 1;
  if (STEPS[n]?.key === "confirm" && skipConfirm()) n -= 1;
  app.stepIndex = Math.max(n, 0);
  app.error = "";
}
// Footer "Continue": import mode persists the phrase before advancing; all
// other steps complete their server work in-step, so this just advances.
export async function continueStep() {
  if (stepKey() === "phrase" && isImport()) {
    const ok = await importWallet();
    if (!ok) return;
  }
  goNext();
}

export function visibleSteps(): number {
  return STEPS.length - (skipConfirm() ? 1 : 0);
}
export function humanIndex(): number {
  return app.stepIndex - (skipConfirm() && app.stepIndex > 2 ? 1 : 0) + 1;
}

export function railClick(i: number) {
  if (i < app.stepIndex && !(STEPS[i].key === "confirm" && skipConfirm())) {
    app.stepIndex = i;
    app.error = "";
  }
}
export function stepState(i: number): "done" | "active" | "skip" | "" {
  if (STEPS[i].key === "confirm" && skipConfirm()) return "skip";
  if (i < app.stepIndex) return "done";
  if (i === app.stepIndex) return "active";
  return "";
}

// ── server actions (real endpoints) ──
export async function refreshHealth() {
  app.serverUp = await client().health();
}

// Local "submitted to OCEAN" marker, keyed by payout address. This is a
// cosmetic completion flag (OCEAN verifies the signature on its own side); the
// signature itself isn't persisted, so if the marker is ever lost the user is
// simply routed to re-sign, which is harmless. Kept in localStorage rather than
// a backend file because it's per-user UX state, not a wallet artifact.
function submittedKey(addr: string): string {
  return `oceanln:submitted:${addr}`;
}
function isSubmitted(addr: string): boolean {
  try {
    return typeof localStorage !== "undefined" && localStorage.getItem(submittedKey(addr)) === "1";
  } catch {
    return false;
  }
}

// Run once on launch, branching on how complete the existing wallet is:
//   - seed + offer → fully set up (the offer file is proof the user finished
//     provisioning), land on the profile. The local "submitted to OCEAN"
//     marker still controls the chip on the profile, but is no longer a gate
//     on landing — it's a local-only acknowledgement and can't be trusted to
//     reflect OCEAN-side state anyway. If the user actually needs to re-sign
//     for OCEAN they can hit "Re-run setup" from the profile.
//   - seed, NO offer → setup was interrupted before provisioning. We can't prove
//     the recovery phrase was backed up (not held in this session), so route to
//     Import: re-supplying the 24 words (idempotent if it matches the stored
//     seed) proves the backup before we provision.
//   - no seed (or unreachable/unauthorized) → fresh onboarding from the top.
// Monotonic bootstrap request id. The `$effect` in App.svelte re-runs
// `bootstrap()` whenever `app.token` or `app.base` changes, so two
// `/status` requests can be in-flight at once. A slow first response
// could otherwise land AFTER a newer credentials-change bootstrap has
// already settled and reset the user back to the previous server's
// state. Same pattern as `StatsGrid.load`.
let bootstrapReqId = 0;

export async function bootstrap() {
  const myId = ++bootstrapReqId;
  let s;
  try {
    s = await client().status();
  } catch {
    /* not configured / unreachable / no token → stay on the wizard */
    if (myId !== bootstrapReqId) return;
    resetWalletIdentity();
    return;
  }
  // Superseded by a newer bootstrap (Settings changed mid-flight).
  // Drop this stale response — the newer call owns the state.
  if (myId !== bootstrapReqId) return;

  if (!s.configured || !s.mining_address) {
    // Fresh install / unreachable / unauthed. If we previously bootstrapped
    // against a configured server and the user has now switched bases/tokens,
    // drop the stale identity so the wizard rebuilds from scratch instead of
    // showing the prior server's address.
    resetWalletIdentity();
    return;
  }
  // Re-bootstrap path: when the user changes base/token in Settings, the
  // existing profile/offer may belong to a different server. Always clear
  // the cached identity before applying the new /status — both
  // `seedProfile()` (early-returns when `app.profile` is set) and the
  // no-offer branch (which would leave a stale `app.offer` intact)
  // otherwise compose with the prior server's data.
  resetWalletIdentity();
  app.miningAddress = s.mining_address;
  if (s.offer) {
    app.offer = s.offer;
    if (!app.offerDescription.trim()) {
      app.offerDescription = `OCEAN Payouts for ${s.mining_address}`;
    }
    app.reuse = true; // skip the create-only reveal/confirm steps
    app.submitted = isSubmitted(s.mining_address); // chip-only; not a gate
    seedProfile();
    app.surface = "profile";
  } else {
    // Require the recovery phrase before provisioning an un-finished wallet.
    app.reuse = false;
    app.mode = "import";
    app.surface = "wizard";
    app.stepIndex = STEPS.findIndex((st) => st.key === "phrase");
  }
}

// Drop any cached identity from a prior bootstrap so a second /status
// result fully replaces (rather than composes with) the first.
//
// Routes away from wallet-only surfaces FIRST (Profile derefs
// `app.profile!.addresses` — clearing while still mounted crashes), then
// wipes BOTH identity fields and any in-flight wizard seed state so a
// subsequent Create on the new server can't accidentally reuse the
// previous session's mnemonic (`Phrase.svelte` only regenerates when
// `app.phrase.length === 0`).
//
// Inlined rather than delegating to `restart()` because `restart()`
// unconditionally resets `app.mode` / `app.stepIndex`, which is too
// aggressive for the in-app "wallet exists" recovery test paths.
// Settings (base, token, accent, density) are NOT touched.
function resetWalletIdentity() {
  if (app.surface === "profile" || app.surface === "dashboard") {
    app.surface = "wizard";
    app.stepIndex = 0;
  }
  // Identity
  app.profile = null;
  app.offer = "";
  app.offerDescription = "";
  app.miningAddress = "";
  app.submitted = false;
  app.reuse = false;
  // Wizard seed state (Codex: the next Create flow would otherwise
  // skip `generateWallet()` and show the prior session's words).
  app.phrase = [];
  app.importWords = Array(24).fill("");
  app.revealed = false;
  app.backedUp = false;
  app.answers = {};
  app.walletExists = false;
  app.signature = "";
  app.message = "";
}

export async function generateWallet() {
  app.busy = true;
  app.error = "";
  try {
    const r = await client().generate();
    app.phrase = r.mnemonic.trim().split(/\s+/);
    app.miningAddress = r.mining_address;
  } catch (e) {
    // A wallet is already configured on this server — recover instead of
    // dead-ending (the create flow has no force-overwrite by design).
    if (e instanceof ApiError && e.status === 409) {
      app.walletExists = true;
    } else {
      app.error = msg(e);
    }
  } finally {
    app.busy = false;
  }
}

// Proceed with the wallet already configured on the server: skip the
// reveal/confirm steps (there's no phrase to show) and go create the offer.
export function useExistingWallet() {
  app.walletExists = false;
  app.reuse = true;
  app.error = "";
  app.stepIndex = STEPS.findIndex((s) => s.key === "wallet");
}

export async function importWallet(): Promise<boolean> {
  app.busy = true;
  app.error = "";
  try {
    const phrase = app.importWords.map((w) => w.trim().toLowerCase()).join(" ");
    const r = await client().importSeed(phrase);
    app.miningAddress = r.mining_address;
    app.phrase = phrase.split(/\s+/);
    return true;
  } catch (e) {
    app.error = msg(e);
    return false;
  } finally {
    app.busy = false;
  }
}

// Provision the wallet + create the offer (both hit the Lexe-backed node).
export async function createWalletAndOffer(): Promise<boolean> {
  app.busy = true;
  app.error = "";
  try {
    const init = await client().init();
    if (init.mining_address) app.miningAddress = init.mining_address;
    // The description is derived from the payout address (no manual step) unless
    // the user set one. Done after /init so the address is definitive (covers
    // the reuse path, where it isn't known until provisioning).
    if (!app.offerDescription.trim()) {
      app.offerDescription = `OCEAN Payouts for ${app.miningAddress}`;
    }
    const offer = await client().offer(app.offerDescription.trim());
    app.offer = offer.offer;
    return true;
  } catch (e) {
    app.error = msg(e);
    return false;
  } finally {
    app.busy = false;
  }
}

// The offer must appear in OCEAN's verification message — same invariant the
// server's offline /payout guard enforces, checked here for a clean UI gate.
export function canSign(): boolean {
  return !!app.offer && app.oceanMessage.trim().length > 0 && app.oceanMessage.includes(app.offer);
}

// Sign the verification message the user pasted from OCEAN (offer-first flow):
// OCEAN issues the message after the user registers their payout address + offer,
// and it embeds the offer. We sign it verbatim via /payout.
export async function signForOcean(): Promise<boolean> {
  if (!canSign()) {
    app.error = "Paste the verification message OCEAN gave you — it must contain your offer.";
    return false;
  }
  app.busy = true;
  app.error = "";
  try {
    const r = await client().payout(app.oceanMessage, app.offer);
    app.message = r.message;
    app.signature = r.signature;
    app.miningAddress = r.address;
    return true;
  } catch (e) {
    app.error = msg(e);
    return false;
  } finally {
    app.busy = false;
  }
}

// Final hand-off. The user copies their address + offer + signature into
// OCEAN's payout settings on ocean.xyz; OCEAN verifies the signature on its
// side. We can't (and shouldn't pretend to) verify it here — this just records
// that the user has submitted, and shows the honest "OCEAN will enable payouts
// once it accepts your signature" success screen.
export function markSubmittedToOcean() {
  app.submitted = true;
  // Persist the completion marker so a relaunch lands on the profile rather than
  // resuming the Sign step (see bootstrap).
  try {
    localStorage?.setItem(submittedKey(app.miningAddress), "1");
  } catch {
    /* localStorage unavailable — non-fatal; worst case is re-signing on restart */
  }
}

// ── post-setup profile ──
export function seedProfile() {
  if (app.profile) return;
  app.profile = {
    offers: [
      { id: "o1", label: app.offerDescription || "OCEAN mining payouts", value: app.offer },
    ],
    addresses: [
      { id: "a1", label: "Primary payout", address: app.miningAddress, offerId: "o1" },
    ],
  };
}
export function go(s: Surface) {
  if (s === "profile") seedProfile();
  app.surface = s;
}
export function restart() {
  app.surface = "wizard";
  app.stepIndex = 0;
  app.mode = "create";
  app.reuse = false;
  app.walletExists = false;
  app.importWords = Array(24).fill("");
  app.revealed = false;
  app.backedUp = false;
  app.answers = {};
  app.offerDescription = "";
  app.phrase = [];
  app.miningAddress = "";
  app.offer = "";
  app.signature = "";
  app.message = "";
  app.oceanMessage = "";
  app.submitted = false;
  app.profile = null;
  app.error = "";
}

function msg(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
