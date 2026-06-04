import { render, screen } from "@testing-library/svelte";
import { beforeEach, describe, it, expect, vi } from "vitest";
import Wallet from "./Wallet.svelte";
import * as S from "../store.svelte";

const ADDR = "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r";
const OFFER = "lno1mockoffer";
const json = (o: unknown) =>
  new Response(JSON.stringify(o), { status: 200, headers: { "content-type": "application/json" } });

beforeEach(() => {
  S.restart();
  S.app.base = "http://x";
  S.app.token = "tok";
  S.app.miningAddress = ADDR;
  globalThis.fetch = vi.fn(async (url: string | URL | Request) => {
    const u = String(url);
    if (u.includes("/init")) return json({ mining_address: ADDR, provisioned: true });
    if (u.includes("/offer")) return json({ offer: OFFER });
    return new Response("not found", { status: 404 });
  }) as typeof fetch;
});

describe("Wallet step — auto offer (no describe step)", () => {
  it("auto-provisions and derives 'OCEAN Payouts for <addr>' (no describe input)", async () => {
    render(Wallet);
    // No manual "Describe your Lightning offer" step.
    expect(screen.queryByText(/Describe your Lightning offer/i)).toBeNull();
    // Auto-creates the offer and shows the payout address.
    expect(await screen.findByText(OFFER)).toBeInTheDocument();
    expect(screen.getByText(ADDR)).toBeInTheDocument();
    // Description derived from the address, no user input.
    expect(S.app.offerDescription).toBe(`OCEAN Payouts for ${ADDR}`);
  });
});
