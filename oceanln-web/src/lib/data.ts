// Static UI content ported from the design's wizard/data.jsx.
// Dashboard payout data is now REAL (fetched live from the OCEAN API — see
// ocean.ts). The MCP block describes a local stdio server (`oceanln mcp serve`)
// that isn't built yet — labelled "coming soon" in the UI.

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
