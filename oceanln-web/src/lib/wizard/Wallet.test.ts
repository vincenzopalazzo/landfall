import { render, screen } from "@testing-library/svelte";
import { beforeEach, describe, it, expect } from "vitest";
import Wallet from "./Wallet.svelte";
import * as S from "../store.svelte";

const ADDR = "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r";

beforeEach(() => S.restart());

describe("Wallet step — input phase", () => {
  it("shows the payout address before the offer is created", () => {
    // Address is derived in the Phrase step (generate/import), so it's known here.
    S.app.miningAddress = ADDR;
    render(Wallet); // app.offer is empty → input phase
    expect(screen.getByText("Your payout address")).toBeInTheDocument();
    expect(screen.getByText(ADDR)).toBeInTheDocument();
    // still the input phase: the description field is present, offer not yet created
    expect(screen.getByPlaceholderText(/OCEAN mining payouts/i)).toBeInTheDocument();
  });
});
