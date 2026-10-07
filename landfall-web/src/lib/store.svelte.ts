import { LandfallClient, ApiError, type Backend } from "./api";
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

  // navigation
  surface: "wizard" as Surface,
  stepIndex: 0,
  mode: "create" as Mode,
  reuse: false, // using a wallet already configured on the server (no phrase to reveal)
  walletExists: false, // a seed is already stored on the server (recover, don't dead-end)
  importConflict: false, // /import hit a DIFFERENT stored seed; awaiting the user's replace/keep choice

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
// base URL / token), HTTP to landfall-httpd in the browser.
export function client(): Backend {
  return isTauri() ? new TauriClient() : new LandfallClient(app.base.replace(/\/$/, ""), app.token);
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
      // Typed answers are only CHECKED on Continue (see `continueStep`), so the
      // button's state never tells a guesser whether a word is right (QA-203).
      return CONFIRM_PICKS.every((_, qi) => (app.answers[qi] ?? "").trim().length > 0);
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
  if (stepKey() === "confirm") {
    if (!answersCorrect()) {
      app.error =
        "One or more words don't match your recovery phrase. Check your written backup and try again.";
      app.answers = {};
      return;
    }
    app.error = "";
  }
  goNext();
}

/// All three typed quiz words equal the phrase at `CONFIRM_PICKS` (case- and
/// whitespace-insensitive). Checked only when the user presses Continue.
export function answersCorrect(): boolean {
  return CONFIRM_PICKS.every((idx, qi) => (app.answers[qi] ?? "").trim().toLowerCase() === app.phrase[idx]);
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
  return `landfall:submitted:${addr}`;
}
// The marker's key before the rename to Landfall, still honoured on read.
function legacySubmittedKey(addr: string): string {
  return `oceanln:submitted:${addr}`;
}
function isSubmitted(addr: string): boolean {
  try {
    return (
      typeof localStorage !== "undefined" &&
      (localStorage.getItem(submittedKey(addr)) === "1" ||
        localStorage.getItem(legacySubmittedKey(addr)) === "1")
    );
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
// Base URL of the last `/status` that answered. A re-bootstrap that FAILS
// against the same base (a token typo mid-edit, a server restart) must not
// wipe the wizard — only an actual move to another server does (QA-212).
let lastGoodBase: string | null = null;

export async function bootstrap() {
  const myId = ++bootstrapReqId;
  // The FIRST bootstrap is the initial page-mount. Any reset here is
  // a no-op for a genuine first-time user (no prior identity exists),
  // and it would clobber state the user may have already produced
  // mid-mount (e.g. clicking "Create a new wallet" before the async
  // /status round-trips and calling generateWallet, populating
  // `app.phrase`). So on the first call we skip the reset on the
  // no-config/error paths entirely. Only *subsequent* bootstraps
  // (Settings change → token/base swap) need to scrub the prior
  // server's leftovers.
  const isRetarget = myId > 1;
  let s;
  try {
    s = await client().status();
  } catch {
    /* not configured / unreachable / no token → stay on the wizard */
    if (myId !== bootstrapReqId) return;
    if (isRetarget && app.base !== lastGoodBase) resetWalletIdentity(true);
    return;
  }
  // Superseded by a newer bootstrap (Settings changed mid-flight).
  // Drop this stale response — the newer call owns the state.
  if (myId !== bootstrapReqId) return;
  lastGoodBase = app.base;

  if (!s.configured || !s.mining_address) {
    // Fresh install / unreachable / unauthed. If we previously bootstrapped
    // against a configured server and the user has now switched bases/tokens,
    // drop the stale identity so the wizard rebuilds from scratch instead of
    // showing the prior server's address.
    if (isRetarget) resetWalletIdentity(true);
    return;
  }
  // First mount, and this session already holds the phrase (the user clicked
  // Create and /generate answered before this /status did): the server's
  // "seed, no offer" is OUR seed mid-setup, not an interrupted earlier run.
  // Resetting here would wipe an un-backed-up phrase.
  if (!isRetarget && app.phrase.length > 0 && !s.offer) return;
  // Re-bootstrap path: when the user changes base/token in Settings, the
  // existing profile/offer may belong to a different server. Always clear
  // the cached identity before applying the new /status — both
  // `seedProfile()` (early-returns when `app.profile` is set) and the
  // no-offer branch (which would leave a stale `app.offer` intact)
  // otherwise compose with the prior server's data. The wizard nav
  // gets rewritten below by either branch, so always allow nav reset here.
  resetWalletIdentity(true);
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
    // Setup was interrupted after the seed was stored but before the offer
    // was created — typically a closed tab before the backup step. The phrase
    // is still on the server, so offer to REVEAL it and resume the backup
    // (Phrase.svelte renders the recovery card when `walletExists` is set)
    // rather than demanding 24 words the user may never have seen (QA-210).
    app.reuse = false;
    app.mode = "create";
    app.walletExists = true;
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
// Settings (base, token, density) are NOT touched.
function resetWalletIdentity(rewindWizard: boolean = false) {
  // Rewind the wizard back to Welcome ONLY when the caller asks
  // (re-bootstrap path). A prior bootstrap that landed on the no-offer
  // Import path leaves `mode === "import"` and `stepIndex` on Phrase
  // with empty words; without resetting both, a re-bootstrap against
  // an unconfigured/401 server strands the user there. But during the
  // FIRST bootstrap (initial page mount) the user may already be
  // mid-click on "Create a new wallet", so resetting nav unconditionally
  // would clobber a freshly-generated phrase.
  if (rewindWizard) {
    app.stepIndex = 0;
    app.mode = "create";
  }
  if (app.surface === "profile" || app.surface === "dashboard") {
    // Profile derefs `app.profile!.addresses`; clearing identity
    // while still mounted crashes mid-render.
    app.surface = "wizard";
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
  app.importConflict = false;
  app.signature = "";
  app.message = "";
  app.oceanMessage = "";
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

// Fetch the stored recovery phrase from the backend for an explicit,
// user-initiated reveal (Profile → "Reveal"). `app.phrase` only survives the
// session that generated/imported it, so a relaunched session re-reads the
// words from the seed source the server already holds (issue #17). Returns
// null on success, or a user-facing error message (e.g. the server runs
// --no-auth and refuses to reveal, or no wallet is configured).
export async function loadPhrase(): Promise<string | null> {
  if (app.phrase.length) return null;
  try {
    const r = await client().revealSeed();
    app.phrase = r.mnemonic.trim().split(/\s+/);
    return null;
  } catch (e) {
    return msg(e);
  }
}

// Resume an interrupted setup: fetch the seed the server already holds
// (`/seed/reveal`) so the user can back it up and confirm it, then continue
// through the normal create flow. Returns false (with `app.error` set) when
// the server refuses — e.g. `--no-auth`, where reveal always needs a token.
export async function recoverStoredPhrase(): Promise<boolean> {
  app.busy = true;
  app.error = "";
  try {
    app.phrase = []; // force a fresh read; `loadPhrase` short-circuits on a cached phrase
    const err = await loadPhrase();
    if (err) {
      app.error = err;
      return false;
    }
    app.walletExists = false;
    app.reuse = false;
    app.mode = "create";
    app.revealed = false;
    app.backedUp = false;
    app.answers = {};
    return true;
  } finally {
    app.busy = false;
  }
}

export async function importWallet(force = false): Promise<boolean> {
  app.busy = true;
  app.error = "";
  app.importConflict = false;
  try {
    const phrase = app.importWords.map((w) => w.trim().toLowerCase()).join(" ");
    const r = await client().importSeed(phrase, force);
    app.miningAddress = r.mining_address;
    app.phrase = phrase.split(/\s+/);
    return true;
  } catch (e) {
    // A DIFFERENT seed is already stored. Never overwrite silently: surface
    // the choice and let `replaceWallet()` retry with force (QA-211).
    if (e instanceof ApiError && e.status === 409 && !force) {
      app.importConflict = true;
      return false;
    }
    app.error = msg(e);
    return false;
  } finally {
    app.busy = false;
  }
}

// The user confirmed the replacement in the conflict card.
export async function replaceWallet(): Promise<boolean> {
  const ok = await importWallet(true);
  if (ok) goNext();
  return ok;
}

// Start the signing step over (wrong message pasted, new OCEAN message, …).
// Clears the signature and the message it covered; address and offer stay.
export function resetSignature() {
  app.signature = "";
  app.message = "";
  app.oceanMessage = "";
  app.error = "";
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
  app.importConflict = false;
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
  // `restart()` is the "I am genuinely starting from zero" entry point
  // (Profile "Re-run setup", test beforeEach). Reset the bootstrap
  // counter so the next bootstrap is treated as a first-time mount,
  // not a re-target — otherwise the test suite (which reuses module
  // state across tests) would see every bootstrap past the first as
  // a retarget and clobber whatever flow the test is exercising.
  bootstrapReqId = 0;
}

function msg(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
