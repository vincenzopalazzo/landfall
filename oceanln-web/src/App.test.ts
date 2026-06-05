import { render, screen, fireEvent } from "@testing-library/svelte";
import { describe, it, expect, beforeEach, vi } from "vitest";
import App from "./App.svelte";
import * as S from "./lib/store.svelte";

const PHRASE =
  "ocean ride lemon harbor velvet crouch target ozone sample dignity market frost april lunar gospel ranch oxygen ribbon kingdom vivid sketch almost dwarf brisk".split(
    " ",
  );
const ADDR = "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r";

function json(o: unknown): Response {
  return new Response(JSON.stringify(o), { status: 200, headers: { "content-type": "application/json" } });
}
function routeFetch(overrides: Record<string, () => Response> = {}) {
  globalThis.fetch = vi.fn(async (url: string | URL | Request) => {
    const u = String(url);
    const path = u.slice(u.lastIndexOf("/"));
    if (overrides[path]) return overrides[path]();
    if (path === "/health") return new Response("{}", { status: 200 });
    if (path === "/status") return json({ configured: false }); // fresh install → wizard
    if (path === "/generate") return json({ mnemonic: PHRASE.join(" "), mining_address: ADDR });
    if (path === "/import") return json({ mining_address: ADDR });
    return new Response("not found", { status: 404 });
  }) as typeof fetch;
}

beforeEach(() => {
  S.restart();
  S.app.base = "http://x";
  S.app.token = "tok";
  routeFetch();
});

describe("App (UI)", () => {
  it("renders the welcome screen with both start choices", () => {
    render(App);
    expect(screen.getByText(/Get paid your mining rewards over Lightning/i)).toBeInTheDocument();
    expect(screen.getByText("Create a new wallet")).toBeInTheDocument();
    expect(screen.getByText("I already have a phrase")).toBeInTheDocument();
  });

  it("create flow advances to the recovery phrase and renders the generated words", async () => {
    render(App);
    await fireEvent.click(screen.getByText("Create a new wallet"));
    expect(await screen.findByText("Your recovery phrase")).toBeInTheDocument();
    expect(await screen.findByText("ocean")).toBeInTheDocument(); // word #1 from /generate
    expect(screen.getByText("brisk")).toBeInTheDocument(); // word #24
  });

  it("gates Continue until the phrase is revealed and backed up", async () => {
    render(App);
    await fireEvent.click(screen.getByText("Create a new wallet"));
    await screen.findByText("ocean");
    expect(screen.getByRole("button", { name: /continue/i })).toBeDisabled();
    await fireEvent.click(screen.getByText(/Tap to reveal/i));
    await fireEvent.click(await screen.findByRole("checkbox"));
    expect(screen.getByRole("button", { name: /continue/i })).not.toBeDisabled();
  });

  it("recovers gracefully when a wallet already exists (409)", async () => {
    routeFetch({ "/generate": () => new Response(JSON.stringify({ error: "seed exists" }), { status: 409 }) });
    render(App);
    await fireEvent.click(screen.getByText("Create a new wallet"));
    expect(await screen.findByText("Use existing wallet")).toBeInTheDocument();
  });

  it("does not loop /generate on a 409 (regression: request-storm guard)", async () => {
    let genCalls = 0;
    routeFetch({
      "/generate": () => {
        genCalls += 1;
        return new Response(JSON.stringify({ error: "seed exists" }), { status: 409 });
      },
    });
    render(App);
    await fireEvent.click(screen.getByText("Create a new wallet"));
    await screen.findByText("Use existing wallet");
    // Let any stray effect re-runs flush; the guard must keep this at exactly one call.
    await new Promise((r) => setTimeout(r, 60));
    expect(genCalls).toBe(1);
  });

  it("import flow renders 24 word inputs", async () => {
    render(App);
    await fireEvent.click(screen.getByText("I already have a phrase"));
    const inputs = await screen.findAllByRole("textbox");
    expect(inputs).toHaveLength(24);
  });
});
