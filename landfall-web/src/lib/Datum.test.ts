import { render, screen } from "@testing-library/svelte";
import { describe, it, expect, beforeEach, vi } from "vitest";
import Datum from "./Datum.svelte";
import * as S from "./store.svelte";
import type { DatumOverview } from "./api";

const ADDR = "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r";

// Shape of landfall-httpd GET /datum/overview (landfall_common::datum::DatumOverview).
function overview(over: Partial<DatumOverview> = {}): DatumOverview {
  return {
    url: "http://192.168.1.10:7152",
    stats: {
      accepted_shares: 1234,
      accepted_shares_diff: 990,
      rejected_shares: 3,
      rejected_shares_diff: 10,
      ready: "Ready",
      pool_host: "datum-beta1.mine.ocean.xyz:28915",
      pool_tag: "OCEAN.XYZ",
      miner_tag: "DATUM User",
      pool_min_diff: 16384,
      pool_pub_key: "f21f2f0ef0aa1970468f22bad9bb7f45",
      uptime: 3 * 86400 + 4 * 3600,
    },
    server: { active_thread: 2, total_connections: 3, total_work_subscriptions: 3, estimated_hashrate: 215.37 },
    job: {
      block_height: 921337,
      block_value: 313512345,
      previous_block: "00000000000000000001a2b3c4d5e6f7",
      block_difficulty: 146472570619930.8,
      limits: { size: 4000000, weight: 4000000, sigops: 80000 },
      size: 1543210,
      weight: 3000000,
      sigops: 12345,
      tx_count: 3456,
      time: { current: 1761900000, minimum: 1761897000 },
    },
    coinbase: [
      { address: "bc1qbigpool", sats: 300000000 },
      { address: ADDR, sats: 13512345 },
    ],
    threads: [{ id: 0, connection_count: 3, subscription_count: 3, approx_hashrate: 215.37 }],
    ...over,
  };
}

function serve(handler: () => Response) {
  const f = vi.fn(async (url: string | URL | Request, _init?: RequestInit) => {
    if (String(url).endsWith("/datum/overview")) return handler();
    return new Response("not found", { status: 404 });
  });
  globalThis.fetch = f as typeof fetch;
  return f;
}
const ok = (body: unknown) =>
  new Response(JSON.stringify(body), { status: 200, headers: { "content-type": "application/json" } });

beforeEach(() => {
  vi.restoreAllMocks();
  S.restart();
  S.app.base = "http://x";
  S.app.token = "tok";
  S.app.datumUrl = "http://192.168.1.10:7152";
  S.app.miningAddress = ADDR;
});

describe("DATUM tab", () => {
  it("renders the gateway snapshot from the backend, never the gateway itself", async () => {
    const f = serve(() => ok(overview()));
    render(Datum);
    expect(await screen.findByText("215.37")).toBeInTheDocument(); // hashrate
    // Ready twice: the header chip and the pool panel's status row.
    expect(screen.getAllByText("Ready")).toHaveLength(2);
    expect(screen.getByText("3d 4h")).toBeInTheDocument(); // uptime
    expect(screen.getByText(/1\.00% by difficulty/)).toBeInTheDocument();
    expect(screen.getByText("datum-beta1.mine.ocean.xyz:28915")).toBeInTheDocument();
    expect(screen.getByText("height 921,337")).toBeInTheDocument();
    expect(screen.getByText("75.0% of limit")).toBeInTheDocument();
    expect(screen.getByText("3.13512345 BTC")).toBeInTheDocument();
    // Only the backend route was called, with the bearer token.
    const urls = f.mock.calls.map((c) => String(c[0]));
    expect(urls.every((u) => u === "http://x/datum/overview")).toBe(true);
    const init = f.mock.calls[0][1] as RequestInit;
    expect((init.headers as Record<string, string>).Authorization).toBe("Bearer tok");
  });

  it("marks the coinbase output that pays the user's own payout address", async () => {
    serve(() => ok(overview()));
    render(Datum);
    expect(await screen.findByText("your payout address")).toBeInTheDocument();
    expect(screen.getAllByText("your payout address")).toHaveLength(1);
  });

  it("says it is waiting when the gateway has no block template yet", async () => {
    serve(() => ok(overview({ job: null })));
    render(Datum);
    expect(await screen.findByText(/Waiting for the first block template/)).toBeInTheDocument();
  });

  it("shows the backend's explanation when the gateway is unreachable", async () => {
    serve(
      () =>
        new Response(
          JSON.stringify({
            error: "DATUM Gateway: could not reach the gateway at http://192.168.1.10:7152 (connection failed). Is it running with `api.listen_port` set in its config?",
          }),
          { status: 502, headers: { "content-type": "application/json" } },
        ),
    );
    render(Datum);
    expect(await screen.findByText("Unreachable")).toBeInTheDocument();
    expect(screen.getByText(/api\.listen_port/)).toBeInTheDocument();
  });

  it("warns when a remote gateway is reached over plain http", async () => {
    S.app.datumUrl = "http://203.0.113.7:7152";
    serve(() => ok(overview({ url: "http://203.0.113.7:7152" })));
    render(Datum);
    expect(await screen.findByText(/travel unencrypted/)).toBeInTheDocument();
  });

  it("does not warn for a LAN gateway", async () => {
    serve(() => ok(overview()));
    render(Datum);
    await screen.findByText("215.37");
    expect(screen.queryByText(/travel unencrypted/)).toBeNull();
  });
});
