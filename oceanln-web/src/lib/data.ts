// Static UI content ported from the design's wizard/data.jsx.
// Dashboard payout data is now REAL (fetched live from the OCEAN API — see
// ocean.ts). The MCP block describes the LIVE read-only MCP server mounted
// at `/mcp` on oceanln-httpd by the `oceanln-mcp` crate — Dashboard.svelte
// renders the actual endpoint URL from `app.base`, this object holds only
// the human-facing tool list + capability copy.

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

/// MCP server presentation.
///
/// PR E (June 2026) split MCP into its OWN process: `oceanln-mcp`
/// runs separately from `oceanln-httpd` and reaches the REST API via
/// HTTP. The dashboard panel just shows the user the URL their MCP
/// client should point at (the standalone binary's port) and reminds
/// them to start the binary. No bearer header is needed at the MCP
/// layer — auth lives between MCP and httpd (`--httpd-token`), not
/// between the MCP client and MCP.
export const MCP = {
  /// Shell command the user runs to start the proxy in another terminal.
  /// `{httpdBase}` is substituted at render time with whatever
  /// `oceanln-httpd` URL the user is pointed at; `{authFlag}` is
  /// substituted with either `` (httpd is in `--no-auth` mode) or
  /// ` --httpd-token <token>` (httpd requires a bearer). Without the
  /// flag, every tool except `get_health` would 401.
  runCmd: "oceanln-mcp --base {httpdBase}{authFlag} --bind 127.0.0.1:7763",
  /// Connect URL for the MCP client (Goose, Claude Code, etc).
  /// Hard-coded to the default `--bind` port; users who change it
  /// have to update accordingly.
  clientUrl: "http://127.0.0.1:7763/mcp",
  tools: [
    "Check wallet status (configured mining address, BOLT12 offer)",
    "List OCEAN Lightning payouts received by your wallet",
    "Fetch OCEAN public stats: unpaid, estimated next-block earnings, hashrate windows",
    "Fetch OCEAN's pool-wide context: active miners, network difficulty, current block reward",
  ],
  cannot:
    "Strictly read-only — every MCP tool is a thin proxy over a REST endpoint on oceanln-httpd. The MCP server itself touches no keys and signs nothing.",
};
