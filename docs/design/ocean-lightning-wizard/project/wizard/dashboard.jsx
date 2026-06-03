// wizard/dashboard.jsx — post-setup payout dashboard + MCP (AI access) panel.
const { useState: useDashState } = React;

function StatusChip({ kind = "ok", pulse, children }) {
  return (
    <span className={`db-chip ${kind}`}>
      <span className={`db-dot ${pulse ? "pulse" : ""}`} />{children}
    </span>
  );
}

function McpPanel({ guided }) {
  const [on, setOn] = useDashState(true);
  return (
    <div className="db-panel db-offercard">
      <div className="db-panel-h">
        <h3><Icon name="link" size={16} style={{ color: "var(--wiz-accent)" }} /> AI access · MCP</h3>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <StatusChip kind={on ? "live" : "muted"} pulse={on}>{on ? "Serving" : "Off"}</StatusChip>
          <button className={`db-toggle ${on ? "on" : ""}`} onClick={() => setOn((v) => !v)} aria-label="Toggle MCP server">
            <span className="knob" />
          </button>
        </div>
      </div>
      <div className="db-panel-b">
        <p className="db-mcp-intro">
          Connect your payout node to <b style={{ color: "#fafafa" }}>Claude Code</b> or any
          MCP-compatible assistant. The AI can read your payout status and help you manage
          your setup — over a local, self-hosted server.
        </p>

        <div style={{ opacity: on ? 1 : 0.4, pointerEvents: on ? "auto" : "none" }}>
          <div className="db-cap-h">Add to Claude Code</div>
          <div className="db-code">
            <code><span className="pre">$ </span>{MCP.addCmd}</code>
            <CopyBtn value={MCP.addCmd} />
          </div>

          <div className="db-cap-h">What the assistant can do</div>
          <ul className="db-caps">
            {MCP.tools.map((t) => (
              <li className="db-cap" key={t}><Icon name="check" size={15} stroke={2} />{t}</li>
            ))}
          </ul>
          <p style={{ fontSize: 12, color: "#52525b", display: "flex", gap: 8, alignItems: "flex-start", margin: "14px 0 0" }}>
            <Icon name="lock" size={13} style={{ marginTop: 1, flexShrink: 0 }} /> {MCP.cannot}
          </p>

          <div className="db-clients">
            <span className="db-dot pulse" style={{ color: "#22c55e" }} />
            1 client connected · Claude Code
          </div>
        </div>
      </div>
    </div>
  );
}

function OfferPanel({ offerDescription }) {
  return (
    <div className="db-panel db-offercard">
      <div className="db-panel-h">
        <h3><Icon name="bolt" size={16} style={{ color: "var(--wiz-accent)" }} /> Your Lightning offer</h3>
        <span className="db-chip muted">BOLT12</span>
      </div>
      <div className="db-panel-b">
        <p className="od">Description</p>
        <p className="ov">{offerDescription || "OCEAN mining payouts"}</p>
        <p className="od">Offer (lno1…)</p>
        <div className="db-code">
          <code style={{ maxHeight: 70, overflow: "hidden" }}>{ARTIFACTS.offer}</code>
          <CopyBtn value={ARTIFACTS.offer} />
        </div>
        <div style={{ height: 12 }} />
        <p className="od">Payout address</p>
        <div className="db-code">
          <code>{ARTIFACTS.address}</code>
          <CopyBtn value={ARTIFACTS.address} />
        </div>
      </div>
    </div>
  );
}

function Dashboard({ guided, offerDescription }) {
  return (
    <React.Fragment>
        <div className="db-top">
          <div>
            <h1>Lightning payouts</h1>
            <p className="sub">Your OCEAN mining rewards, paid over Lightning.</p>
          </div>
          <StatusChip kind="ok" pulse>Node online</StatusChip>
        </div>

        <div className="db-stats">
          <div className="db-stat">
            <div className="l"><Icon name="bolt" size={12} /> Pending balance</div>
            <div className="v">{DASH.pendingSats} <span className="u">sats</span></div>
          </div>
          <div className="db-stat">
            <div className="l"><Icon name="wallet" size={12} /> Total received</div>
            <div className="v">{DASH.totalBtc} <span className="u">BTC</span></div>
          </div>
          <div className="db-stat">
            <div className="l"><Icon name="check" size={12} /> Payouts</div>
            <div className="v">{DASH.payoutsCount}</div>
          </div>
          <div className="db-stat">
            <div className="l"><Icon name="refresh" size={12} /> Next payout</div>
            <div className="v accent">{DASH.nextEta}</div>
          </div>
        </div>

        <div className="db-panel">
          <div className="db-panel-h">
            <h3>Recent payouts</h3>
            <span className="meta">{DASH.node.enclave} · up {DASH.node.uptime}</span>
          </div>
          <table className="db-table">
            <thead>
              <tr><th>Time</th><th>Status</th><th className="r">Amount</th></tr>
            </thead>
            <tbody>
              {DASH.payouts.map((p, i) => (
                <tr key={i}>
                  <td>{p.time}</td>
                  <td>
                    <span className={`db-status ${p.status === "settled" ? "settled" : "inflight"}`}>
                      <span className="db-dot" />{p.status === "settled" ? "Settled" : "In-flight"}
                    </span>
                  </td>
                  <td className="r amt">{p.sats === "—" ? "—" : `${p.sats} sats`}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div className="db-grid2">
          <OfferPanel offerDescription={offerDescription} />
          <McpPanel guided={guided} />
        </div>
    </React.Fragment>
  );
}

Object.assign(window, { Dashboard });
