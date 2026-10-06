import { describe, it, expect, vi, beforeEach } from "vitest";
import * as S from "./store.svelte";

const { app } = S;

const PHRASE =
  "ocean ride lemon harbor velvet crouch target ozone sample dignity market frost april lunar gospel ranch oxygen ribbon kingdom vivid sketch almost dwarf brisk".split(
    " ",
  );
const ADDR = "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r";
const OFFER = "lno1mockoffer";
const SIG = "AkcwRAIgmocksignature==";

function json(o: unknown): Response {
  return new Response(JSON.stringify(o), { status: 200, headers: { "content-type": "application/json" } });
}

function routeFetch(overrides: Record<string, () => Response> = {}) {
  globalThis.fetch = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
    const u = String(url);
    const path = u.slice(u.lastIndexOf("/"));
    if (overrides[path]) return overrides[path]();
    if (path === "/health") return new Response("{}", { status: 200 });
    if (path === "/generate") return json({ mnemonic: PHRASE.join(" "), mining_address: ADDR });
    if (path === "/import") return json({ mining_address: ADDR });
    if (path === "/init") return json({ mining_address: ADDR, provisioned: true });
    if (path === "/offer") return json({ offer: OFFER });
    if (path === "/payout") {
      const b = JSON.parse(init!.body as string);
      return json({ address: ADDR, offer: b.offer, message: b.message, signature: SIG });
    }
    return new Response("not found", { status: 404 });
  }) as typeof fetch;
}

beforeEach(() => {
  S.restart();
  try {
    localStorage.clear();
  } catch {
    /* jsdom always has localStorage; guard anyway */
  }
  app.base = "http://x";
  app.token = "tok";
  routeFetch();
});

describe("wizard happy path (create)", () => {
  it("drives generate → confirm → wallet → sign → done with real artifacts", async () => {
    S.chooseMode("create");
    expect(S.stepKey()).toBe("phrase");

    await S.generateWallet();
    expect(app.phrase).toHaveLength(24);
    expect(app.miningAddress).toBe(ADDR);
    expect(S.canContinue()).toBe(false); // not revealed/backed up yet

    app.revealed = true;
    app.backedUp = true;
    expect(S.canContinue()).toBe(true);

    await S.continueStep();
    expect(S.stepKey()).toBe("confirm");
    expect(S.canContinue()).toBe(false);
    // answer the quiz correctly
    S.CONFIRM_PICKS.forEach((idx, qi) => (app.answers[qi] = app.phrase[idx]));
    expect(S.canContinue()).toBe(true);

    await S.continueStep();
    expect(S.stepKey()).toBe("wallet");
    expect(S.canContinue()).toBe(false); // no offer yet

    app.offerDescription = "OCEAN mining payouts";
    const ok = await S.createWalletAndOffer();
    expect(ok).toBe(true);
    expect(app.offer).toBe(OFFER);
    expect(S.canContinue()).toBe(true);

    await S.continueStep();
    expect(S.stepKey()).toBe("sign");
    // Can't sign until OCEAN's message (embedding the offer) is pasted.
    expect(S.canSign()).toBe(false);
    app.oceanMessage = `Authorize payout to ${ADDR} via ${OFFER}`;
    expect(S.canSign()).toBe(true);
    const signed = await S.signForOcean();
    expect(signed).toBe(true);
    expect(app.signature).toBe(SIG);
    expect(app.message).toContain(OFFER); // the pasted message is signed verbatim
    expect(app.message).toContain(ADDR);
    expect(S.canContinue()).toBe(true);

    await S.continueStep();
    expect(S.stepKey()).toBe("done");
  });
});

describe("wizard import path", () => {
  it("imports a phrase and skips the confirm step", async () => {
    S.chooseMode("import");
    expect(S.stepKey()).toBe("phrase");
    expect(S.canContinue()).toBe(false); // empty inputs

    PHRASE.forEach((w, i) => (app.importWords[i] = w));
    expect(S.canContinue()).toBe(true);

    await S.continueStep(); // imports, then advances past confirm
    expect(app.miningAddress).toBe(ADDR);
    expect(S.stepKey()).toBe("wallet"); // confirm skipped for imports
    expect(S.visibleSteps()).toBe(5); // confirm hidden
  });
});

describe("error handling", () => {
  it("recovers from a 409 (wallet exists) instead of dead-ending", async () => {
    routeFetch({ "/generate": () => new Response(JSON.stringify({ error: "seed file exists" }), { status: 409 }) });
    S.chooseMode("create");
    await S.generateWallet();
    // No dead-end error; flagged as an existing wallet to recover from.
    expect(app.walletExists).toBe(true);
    expect(app.error).toBe("");
    expect(app.phrase).toHaveLength(0);

    // "Use existing wallet" jumps to the wallet step, skipping reveal/confirm.
    S.useExistingWallet();
    expect(S.stepKey()).toBe("wallet");
    expect(S.visibleSteps()).toBe(5); // confirm hidden for a reused wallet
  });

  it("surfaces a wallet/offer failure and does not advance", async () => {
    routeFetch({ "/offer": () => new Response(JSON.stringify({ error: "lexe unreachable" }), { status: 502 }) });
    app.offerDescription = "x";
    const ok = await S.createWalletAndOffer();
    expect(ok).toBe(false);
    expect(app.error).toContain("lexe unreachable");
    expect(app.offer).toBe("");
  });
});

describe("sign gating", () => {
  it("refuses to sign a message that doesn't embed the offer", async () => {
    app.offer = "lno1realoffer";
    app.oceanMessage = "a message that forgot the offer";
    expect(S.canSign()).toBe(false);
    const ok = await S.signForOcean();
    expect(ok).toBe(false);
    expect(app.error).toMatch(/must contain your offer/i);
    expect(app.signature).toBe("");
  });
});

describe("bootstrap (skip the wizard when a wallet exists)", () => {
  it("lands on the profile when configured + offer + already submitted (chip on)", async () => {
    localStorage.setItem(`oceanln:submitted:${ADDR}`, "1"); // completed OCEAN hand-off
    routeFetch({ "/status": () => json({ configured: true, mining_address: ADDR, offer: OFFER }) });
    expect(app.surface).toBe("wizard");
    await S.bootstrap();
    expect(app.surface).toBe("profile");
    expect(app.miningAddress).toBe(ADDR);
    expect(app.offer).toBe(OFFER);
    expect(app.profile).not.toBeNull();
    expect(app.submitted).toBe(true);
  });

  it("ALSO lands on the profile when offer exists but submitted-chip is off", async () => {
    // Presence of the offer file is proof the user finished provisioning;
    // the local "submitted" marker is just a chip, not a gate. The escape
    // hatch for re-doing OCEAN verification is "Re-run setup" on the profile.
    routeFetch({ "/status": () => json({ configured: true, mining_address: ADDR, offer: OFFER }) });
    await S.bootstrap();
    expect(app.surface).toBe("profile");
    expect(app.miningAddress).toBe(ADDR);
    expect(app.offer).toBe(OFFER);
    expect(app.profile).not.toBeNull();
    expect(app.submitted).toBe(false); // chip stays off until they sign
  });

  it("QA-210: offers to reveal + back up the stored phrase when configured but no offer yet", async () => {
    // Setup was interrupted (seed exists, /offer never completed) — typically a
    // closed tab before the backup. The phrase is on the server, so the wizard
    // must offer to reveal it (Phrase.svelte recovery card), not demand 24 words
    // the user may never have seen, and must not skip the backup.
    routeFetch({ "/status": () => json({ configured: true, mining_address: ADDR, offer: null }) });
    await S.bootstrap();
    expect(app.surface).toBe("wizard");
    expect(S.stepKey()).toBe("phrase");
    expect(S.isImport()).toBe(false);
    expect(app.walletExists).toBe(true); // recovery card, not the import grid
    expect(app.reuse).toBe(false); // backup/confirm not skipped
    expect(app.miningAddress).toBe(ADDR);
    expect(app.offer).toBe("");

    // "Reveal and back up" reads the stored phrase and resumes the create flow.
    routeFetch({
      "/status": () => json({ configured: true, mining_address: ADDR, offer: null }),
      "/reveal": () => json({ mnemonic: PHRASE.join(" ") }),
    });
    expect(await S.recoverStoredPhrase()).toBe(true);
    expect(app.phrase).toEqual(PHRASE);
    expect(app.walletExists).toBe(false);
    expect(app.revealed).toBe(false); // the user still has to tap-to-reveal and tick the box
    expect(S.canContinue()).toBe(false);
  });

  it("QA-210: a refused reveal (e.g. --no-auth) is an error on the card, not a dead end", async () => {
    routeFetch({
      "/status": () => json({ configured: true, mining_address: ADDR, offer: null }),
      "/reveal": () => new Response("requires auth", { status: 403 }),
    });
    await S.bootstrap();
    expect(await S.recoverStoredPhrase()).toBe(false);
    expect(app.walletExists).toBe(true); // card stays, with the error and the "continue anyway" path
    expect(app.error).not.toBe("");
  });

  it("QA-212: a failed re-bootstrap against the SAME base keeps an un-backed-up phrase", async () => {
    routeFetch({ "/status": () => json({ configured: false }) });
    await S.bootstrap(); // first mount
    S.chooseMode("create");
    await S.generateWallet();
    expect(app.phrase).toEqual(PHRASE);
    // Token edited to a wrong value → 401 on the same server: not a move.
    routeFetch({ "/status": () => new Response("nope", { status: 401 }) });
    await S.bootstrap();
    expect(app.phrase).toEqual(PHRASE);
    expect(S.stepKey()).toBe("phrase");
    // An actual move to another base does reset.
    app.base = "http://elsewhere";
    await S.bootstrap();
    expect(app.phrase).toEqual([]);
    expect(S.stepKey()).toBe("welcome");
  });

  it("first-mount /status landing after /generate does not wipe the fresh phrase", async () => {
    // Race: user clicks Create and /generate answers before the mount-time
    // /status does; the server now reports "seed, no offer" — which is OUR seed.
    S.chooseMode("create");
    await S.generateWallet();
    routeFetch({ "/status": () => json({ configured: true, mining_address: ADDR, offer: null }) });
    await S.bootstrap();
    expect(app.phrase).toEqual(PHRASE);
    expect(app.walletExists).toBe(false);
    expect(S.stepKey()).toBe("phrase");
  });

  it("stays on the wizard for a fresh install (not configured)", async () => {
    routeFetch({ "/status": () => json({ configured: false }) });
    await S.bootstrap();
    expect(app.surface).toBe("wizard");
  });

  it("stays on the wizard if the server is unreachable", async () => {
    routeFetch({ "/status": () => new Response("nope", { status: 401 }) });
    await S.bootstrap();
    expect(app.surface).toBe("wizard");
  });
});

describe("OCEAN hand-off", () => {
  it("markSubmittedToOcean is a local ack, not a fake network verify", () => {
    expect(app.submitted).toBe(false);
    S.markSubmittedToOcean();
    // Flips synchronously — no setTimeout, no pretended "verifying" round-trip.
    expect(app.submitted).toBe(true);
    S.restart();
    expect(app.submitted).toBe(false);
  });
});

describe("navigation math", () => {
  it("create flow shows 6 steps, import shows 5", () => {
    S.chooseMode("create");
    expect(S.visibleSteps()).toBe(6);
    S.restart();
    S.chooseMode("import");
    expect(S.visibleSteps()).toBe(5);
  });
});

describe("QA-211: importing over a different stored wallet", () => {
  it("surfaces a replace/keep choice on 409 and replaces only with force", async () => {
    let forced: boolean | null = null;
    globalThis.fetch = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
      const path = String(url).slice(String(url).lastIndexOf("/"));
      if (path === "/import") {
        forced = JSON.parse(init!.body as string).force === true;
        return forced
          ? json({ mining_address: ADDR })
          : new Response("seed file exists with a different seed", { status: 409 });
      }
      if (path === "/health") return new Response("{}", { status: 200 });
      return new Response("not found", { status: 404 });
    }) as typeof fetch;
    S.chooseMode("import");
    app.importWords = [...PHRASE];
    await S.continueStep();
    expect(app.importConflict).toBe(true);
    expect(app.error).toBe(""); // a choice, not an error
    expect(S.stepKey()).toBe("phrase"); // did not advance
    expect(forced).toBe(false);

    app.importConflict = false; // "Keep the existing wallet"
    await S.continueStep();
    expect(app.importConflict).toBe(true); // asked again, still nothing replaced

    expect(await S.replaceWallet()).toBe(true);
    expect(forced).toBe(true);
    expect(app.importConflict).toBe(false);
    expect(app.miningAddress).toBe(ADDR);
    expect(S.stepKey()).toBe("wallet"); // import skips confirm
  });
});

describe("QA-203: typed backup check, judged only on Continue", () => {
  async function toConfirm() {
    S.chooseMode("create");
    await S.generateWallet();
    app.revealed = true;
    app.backedUp = true;
    await S.continueStep();
    expect(S.stepKey()).toBe("confirm");
  }
  it("Continue is enabled once all three words are typed, whatever they are", async () => {
    await toConfirm();
    expect(S.canContinue()).toBe(false);
    app.answers = { 0: "wrong", 1: "words", 2: "typed" };
    expect(S.canContinue()).toBe(true); // no per-word verdict leaks before Continue
  });
  it("a wrong word is rejected on Continue, the answers are cleared, the step stays", async () => {
    await toConfirm();
    app.answers = { 0: PHRASE[S.CONFIRM_PICKS[0]], 1: "wrong", 2: PHRASE[S.CONFIRM_PICKS[2]] };
    await S.continueStep();
    expect(S.stepKey()).toBe("confirm");
    expect(app.error).toMatch(/don't match/);
    expect(app.answers).toEqual({});
  });
  it("the right words (any case / spacing) advance to the wallet step", async () => {
    await toConfirm();
    app.answers = {
      0: ` ${PHRASE[S.CONFIRM_PICKS[0]].toUpperCase()} `,
      1: PHRASE[S.CONFIRM_PICKS[1]],
      2: PHRASE[S.CONFIRM_PICKS[2]],
    };
    await S.continueStep();
    expect(S.stepKey()).toBe("wallet");
    expect(app.error).toBe("");
  });
});

describe("QA-213: the sign step can be redone", () => {
  it("resetSignature clears the signature and both message copies, keeps address + offer", async () => {
    app.offer = OFFER;
    app.miningAddress = ADDR;
    app.oceanMessage = `Configure OCEAN payout to ${OFFER} at block 1`;
    expect(await S.signForOcean()).toBe(true);
    expect(app.signature).toBe(SIG);
    expect(app.message).toBe(app.oceanMessage); // what was signed is kept for display
    S.resetSignature();
    expect(app.signature).toBe("");
    expect(app.message).toBe("");
    expect(app.oceanMessage).toBe("");
    expect(app.offer).toBe(OFFER);
    expect(app.miningAddress).toBe(ADDR);
    expect(S.canSign()).toBe(false);
  });
});
