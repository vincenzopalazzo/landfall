import { render, screen } from "@testing-library/svelte";
import { describe, it, expect, beforeEach, vi } from "vitest";
import Dashboard from "./Dashboard.svelte";
import * as S from "./store.svelte";

function res(body: unknown): Response {
  return new Response(JSON.stringify(body), { status: 200, headers: { "content-type": "application/json" } });
}

// Real node_status shape (oceanln-httpd GET /node).
const NODE = {
  node_pk: "02abcdef",
  num_channels: 2,
  num_usable_channels: 2,
  lightning_total_sats: 1284503,
  lightning_sendable_sats: 1000000,
  onchain_total_sats: 3812000,
  onchain_trusted_sats: 3812000,
  total_balance_sats: 5096503,
};

// Real activity shape (GET /activity): one verified OCEAN payout, one
// non-OCEAN inbound tip, one outbound send.
const ACTS = [
  { id: "a1", direction: "in", rail: "ln", amount_sats: 12084, amount_msat: 12084000, status: "settled", note: "OCEAN lightning payout running at block `00` at height `897142`", counterparty: "Ocean Pool", finalized_at_ms: 1717500000000, payment_hash: "aa", txid: null, is_ocean: true, block_height: 897142 },
  { id: "a2", direction: "in", rail: "ln", amount_sats: 25000, amount_msat: 25000000, status: "settled", note: "Tip", counterparty: "satoshi@walletofsatoshi.com", finalized_at_ms: 1717400000000, payment_hash: "bb", txid: null, is_ocean: false, block_height: null },
  { id: "a3", direction: "out", rail: "ln", amount_sats: 150000, amount_msat: 150000000, status: "settled", note: null, counterparty: "Kraken", finalized_at_ms: 1717300000000, payment_hash: "cc", txid: null, is_ocean: false, block_height: null },
];

function routeAll() {
  globalThis.fetch = vi.fn(async (url: string | URL | Request) => {
    const u = String(url);
    if (u.endsWith("/health")) return new Response("ok", { status: 200 });
    if (u.endsWith("/node")) return res(NODE);
    if (u.includes("/activity")) return res(ACTS);
    if (u.endsWith("/payouts") || u.includes("/payouts?")) return res([]);
    if (u.includes("mempool.space")) return res({ USD: 100000 });
    return new Response("not found", { status: 404 });
  }) as typeof fetch;
}

beforeEach(() => {
  vi.restoreAllMocks();
  S.restart();
  S.app.serverUp = true;
  S.app.offer = "lno1testoffer";
  S.app.offerDescription = "OCEAN mining payouts";
  S.app.miningAddress = "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r";
  routeAll();
});

describe("Lightning dashboard — Node wallet", () => {
  it("renders the header, node-online chip, and the Node wallet bar", async () => {
    render(Dashboard);
    expect(await screen.findByText("Lightning payouts")).toBeInTheDocument();
    expect(await screen.findByText("Node online")).toBeInTheDocument();
    expect(screen.getByText("Node wallet")).toBeInTheDocument();
    expect(screen.getByText("Receive")).toBeInTheDocument();
    expect(screen.getByText("Send")).toBeInTheDocument();
  });

  it("shows live balance cards from node_status", async () => {
    render(Dashboard);
    // Channel capacity legend is a single clean text node (avoids the
    // value/unit span split on the big number).
    expect(await screen.findByText("1,000,000 spendable")).toBeInTheDocument();
    expect(screen.getByText("Lightning channel")).toBeInTheDocument();
    expect(screen.getByText("On-chain")).toBeInTheDocument();
    expect(screen.getByText("Total received")).toBeInTheDocument();
  });

  it("lists node activity with OCEAN detection and grouping", async () => {
    render(Dashboard);
    expect(await screen.findByText("Node activity")).toBeInTheDocument();
    // The verified OCEAN payout, the non-OCEAN tip, and the outbound send.
    expect(await screen.findByText("Ocean Pool")).toBeInTheDocument();
    expect(screen.getByText("satoshi@walletofsatoshi.com")).toBeInTheDocument();
    expect(screen.getByText("Kraken")).toBeInTheDocument();
    // "OCEAN payouts" appears as a filter chip and the group subheader.
    expect(screen.getAllByText("OCEAN payouts").length).toBeGreaterThanOrEqual(1);
    // The verified row carries the OCEAN-payout chip.
    expect(screen.getByText("OCEAN payout")).toBeInTheDocument();
  });

  it("still shows the offer panel and the AI access (MCP) panel", async () => {
    render(Dashboard);
    expect(await screen.findByText("Your Lightning offer")).toBeInTheDocument();
    expect(screen.getByText("AI access · MCP")).toBeInTheDocument();
    expect(screen.getByText("lno1testoffer")).toBeInTheDocument();
  });

  it("reads 'Node unreachable' when the backend health probe is down", async () => {
    globalThis.fetch = vi.fn(async (url: string | URL | Request) => {
      const u = String(url);
      if (u.endsWith("/health")) return new Response("down", { status: 503 });
      if (u.endsWith("/node")) return res(NODE);
      if (u.includes("/activity")) return res([]);
      if (u.includes("mempool.space")) return res({ USD: 100000 });
      return new Response("nf", { status: 404 });
    }) as typeof fetch;
    render(Dashboard);
    expect(await screen.findByText("Node unreachable")).toBeInTheDocument();
  });
});
