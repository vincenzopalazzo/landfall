// Static content ported from the design's wizard/data.jsx.
// The DASH and MCP blocks are ILLUSTRATIVE mocks — the backend has no payout
// monitoring or MCP server yet (clearly labelled in the UI).

export const WORD_POOL = [
  "anchor", "tide", "copper", "signal", "forest", "cabin", "jacket", "puzzle",
  "meadow", "walnut", "rocket", "candle", "pigeon", "saddle", "quartz", "echo",
  "marble", "ginger", "pottery", "silver", "tunnel", "cobalt", "beacon", "lantern",
  "harvest", "drift", "ember", "glacier", "thistle", "cargo", "ladder", "orbit",
];

export const OFFER_SUGGESTIONS = [
  "OCEAN mining payouts",
  "Rig 01 · Lightning rewards",
  "Home miner payouts",
];

export const PROVISION_TASKS = [
  { t: "Starting your Lightning wallet", s: "Spinning up your node in a secure enclave (Lexe)" },
  { t: "Creating your Lightning address", s: "Embedding your description into a BOLT12 offer (lno1…)" },
  { t: "Deriving your payout address", s: "Native SegWit Bitcoin address (bc1q…)" },
];

export interface DashData {
  node: { status: string; enclave: string; uptime: string; peers: number };
  pendingSats: string;
  totalBtc: string;
  payoutsCount: number;
  nextEta: string;
  payouts: { time: string; sats: string; status: "settled" | "in-flight" }[];
}

export const DASH: DashData = {
  node: { status: "Online", enclave: "SGX enclave · us-east-1", uptime: "4d 02h", peers: 6 },
  pendingSats: "18,420",
  totalBtc: "0.04123",
  payoutsCount: 7,
  nextEta: "~6h 12m",
  payouts: [
    { time: "2026-06-03 13:58", sats: "—", status: "in-flight" },
    { time: "2026-06-03 09:14", sats: "12,084", status: "settled" },
    { time: "2026-06-02 21:50", sats: "9,732", status: "settled" },
    { time: "2026-06-02 08:31", sats: "11,640", status: "settled" },
    { time: "2026-06-01 19:07", sats: "8,905", status: "settled" },
    { time: "2026-06-01 06:42", sats: "10,318", status: "settled" },
  ],
};

export const MCP = {
  addCmd: "claude mcp add oceanln -- oceanln mcp serve",
  tools: [
    "Check your payout status and history",
    "Read your BOLT12 offer and payout address",
    "Report node health and pending balance",
    "Re-sign OCEAN's verification message on request",
  ],
  cannot: "It runs read-only by default and can never move or spend your funds.",
};
