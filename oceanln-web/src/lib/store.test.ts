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

describe("navigation math", () => {
  it("create flow shows 6 steps, import shows 5", () => {
    S.chooseMode("create");
    expect(S.visibleSteps()).toBe(6);
    S.restart();
    S.chooseMode("import");
    expect(S.visibleSteps()).toBe(5);
  });
});
