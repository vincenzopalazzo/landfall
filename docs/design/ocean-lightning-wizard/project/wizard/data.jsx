// wizard/data.jsx — static content + mock crypto artifacts for the OCEAN Lightning wizard.
// All values are illustrative (prototype only); none are real keys/offers.

// A fixed 24-word demo recovery phrase (valid BIP39 words).
const PHRASE_24 = [
  "ocean", "ride", "lemon", "harbor", "velvet", "crouch",
  "target", "ozone", "sample", "dignity", "market", "frost",
  "april", "lunar", "gospel", "ranch", "oxygen", "ribbon",
  "kingdom", "vivid", "sketch", "almost", "dwarf", "brisk",
];

// Decoy pool for the confirm step's multiple-choice.
const WORD_POOL = [
  "anchor","tide","copper","signal","forest","cabin","jacket","puzzle",
  "meadow","walnut","rocket","candle","pigeon","saddle","quartz","echo",
  "marble","ginger","pottery","silver","tunnel","cobalt","beacon","lantern",
  "harvest","drift","ember","glacier","thistle","cargo","ladder","orbit",
];

// The three artifacts the wizard produces (mock).
const ARTIFACTS = {
  address: "bc1q9x7k2m4p8v3wq5r6t7y8u9i0a2s3d4f5g6h7j",
  offer:
    "lno1pgsqvgnwgcg35z6ee2h3yczraddm72xrfua9uve2rlrm9deu7xyfzr" +
    "5g3qfqu94yqsqd9hy6tq2qg3w0pmza4f4kqg83r9ftz29x0vqa8s2k4mfl" +
    "ph6hq3kq7n0w5z2u4cdy3p9ka8w3hxu",
  signature:
    "AkcwRAIgY8b3pNq1Ld5fK0vRr2xH9wEoP6mZ2sT1uVcXa7nQ8kCIB4tLm" +
    "0sJ9dRfWpE3yU7vGqHbN6cZ1xK5oA2wD4eRtY=",
};

// The verification message OCEAN asks the miner to sign (mock, with nonce).
const OCEAN_MESSAGE =
  "OCEAN Lightning Payout Authorization\n" +
  "Pool: ocean.xyz\n" +
  "Payout address: bc1q9x7…6h7j\n" +
  "Issued: 2026-06-03T14:22:09Z\n" +
  "Nonce: a3f9c1e8-72b4-4d10-9c6e-1f55ed90b3a2";

// Step rail definitions.
const STEPS = [
  { key: "welcome",  label: "Welcome" },
  { key: "phrase",   label: "Recovery phrase" },
  { key: "confirm",  label: "Confirm backup" },
  { key: "wallet",   label: "Create wallet" },
  { key: "sign",     label: "Sign for OCEAN" },
  { key: "done",     label: "Turn on payouts" },
];

// Provisioning sub-tasks (create-wallet step).
const PROVISION_TASKS = [
  { t: "Starting your Lightning wallet",   s: "Spinning up your node in a secure enclave (Lexe)" },
  { t: "Creating your Lightning address",  s: "Embedding your description into a BOLT12 offer (lno1…)" },
  { t: "Deriving your payout address",     s: "Native SegWit Bitcoin address (bc1q…)" },
];

// Suggested labels for the offer description field.
const OFFER_SUGGESTIONS = ["OCEAN mining payouts", "Rig 01 · Lightning rewards", "Home miner payouts"];

// ── Dashboard (payout monitoring) mock data ──
const DASH = {
  node:        { status: "Online", enclave: "SGX enclave · us-east-1", uptime: "4d 02h", peers: 6 },
  pendingSats: "18,420",
  totalBtc:    "0.04123",
  payoutsCount: 7,
  nextEta:     "~6h 12m",
  payouts: [
    { time: "2026-06-03 13:58", sats: "—",      status: "in-flight" },
    { time: "2026-06-03 09:14", sats: "12,084", status: "settled" },
    { time: "2026-06-02 21:50", sats: "9,732",  status: "settled" },
    { time: "2026-06-02 08:31", sats: "11,640", status: "settled" },
    { time: "2026-06-01 19:07", sats: "8,905",  status: "settled" },
    { time: "2026-06-01 06:42", sats: "10,318", status: "settled" },
  ],
};

// ── MCP / AI access config (connect node to Claude Code or any MCP client) ──
const MCP = {
  addCmd:   "claude mcp add oceanln -- oceanln mcp serve",
  endpoint: "oceanln mcp serve  ·  stdio",
  tools: [
    "Check your payout status and history",
    "Read your BOLT12 offer and payout address",
    "Report node health and pending balance",
    "Re-sign OCEAN's verification message on request",
  ],
  cannot: "It runs read-only by default and can never move or spend your funds.",
};

Object.assign(window, {
  PHRASE_24, WORD_POOL, ARTIFACTS, OCEAN_MESSAGE, STEPS, PROVISION_TASKS,
  OFFER_SUGGESTIONS, DASH, MCP,
});
