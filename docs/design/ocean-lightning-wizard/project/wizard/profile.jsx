// wizard/profile.jsx — post-setup app shell (nav) + Profile page.
const { useState: useProfState } = React;

const NODE_ID = "03f9a4c1d7e8…b21c";

// ── Shell: top nav (Profile / Lightning) + scroll area ──
function AppShell({ active, go, children }) {
  return (
    <div className="wz-content">
      <div className="app-nav">
        <span className="brand"><img src="assets/OCEAN-icon-white.svg" alt="OCEAN" /></span>
        <button className={`pill ${active === "profile" ? "active" : ""}`} onClick={() => go("profile")}>
          <Icon name="wallet" size={15} /> Profile
        </button>
        <button className={`pill ${active === "dashboard" ? "active" : ""}`} onClick={() => go("dashboard")}>
          <Icon name="bolt" size={15} /> Lightning
        </button>
        <span className="spacer" />
        <span className="acct"><span className="av">M</span> OCEAN Miner</span>
      </div>
      <div className="app-scroll">{children}</div>
    </div>
  );
}

const rhex = (n) => Array.from({ length: n }, () => "0123456789abcdef"[Math.floor(Math.random() * 16)]).join("");

function Profile({ guided, phrase, profile, setProfile, go, onReturnToSetup }) {
  const [revealed, setRevealed] = useProfState(false);
  const { addresses, offers } = profile;

  const addAddress = () => setProfile((p) => ({
    ...p,
    addresses: [...p.addresses, {
      id: "a" + Date.now(), label: "Payout address " + (p.addresses.length + 1),
      address: "bc1q" + rhex(38), offerId: p.offers[0] ? p.offers[0].id : null,
    }],
  }));
  const addOffer = () => setProfile((p) => ({
    ...p,
    offers: [...p.offers, { id: "o" + Date.now(), label: "Offer " + (p.offers.length + 1), value: "lno1" + rhex(92) }],
  }));
  const linkOffer = (addrId, offerId) => setProfile((p) => ({
    ...p, addresses: p.addresses.map((a) => (a.id === addrId ? { ...a, offerId: offerId || null } : a)),
  }));
  const offerLabel = (id) => { const o = offers.find((x) => x.id === id); return o ? o.label : "—"; };
  const linkCount = (id) => addresses.filter((a) => a.offerId === id).length;

  return (
    <React.Fragment>
      <div className="pf-head">
        <div className="pf-av">M</div>
        <div>
          <h1 className="pf-name">OCEAN Miner</h1>
          <p className="pf-id">
            Node {NODE_ID}
            <span className="db-chip ok"><span className="db-dot pulse" /> Verified</span>
          </p>
        </div>
      </div>

      {/* Onchain addresses */}
      <div className="pf-section">
        <div className="pf-section-h">
          <h2><Icon name="key" size={15} /> Onchain payout addresses</h2>
          <button className="pf-add" onClick={addAddress}><Icon name="key" size={13} /> Derive address</button>
        </div>
        <div className="pf-list">
          {addresses.map((a) => (
            <div className="pf-addr" key={a.id}>
              <div className="ic"><Icon name="key" size={16} /></div>
              <div className="meta">
                <p className="lbl">{a.label}</p>
                <p className="val">{a.address}</p>
              </div>
              <div className="link">
                <span className="pf-link-lbl">Linked offer</span>
                <select className="pf-select" value={a.offerId || ""} onChange={(e) => linkOffer(a.id, e.target.value)}>
                  <option value="">Not linked</option>
                  {offers.map((o) => <option key={o.id} value={o.id}>{o.label}</option>)}
                </select>
                <CopyBtn value={a.address} label="" />
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Lightning offers */}
      <div className="pf-section">
        <div className="pf-section-h">
          <h2><Icon name="bolt" size={15} /> Lightning offers</h2>
          <button className="pf-add" onClick={addOffer}><Icon name="bolt" size={13} /> New offer</button>
        </div>
        <div className="pf-list">
          {offers.map((o) => (
            <div className="pf-offerrow" key={o.id}>
              <div className="ic"><Icon name="bolt" size={15} /></div>
              <div className="meta">
                <p className="lbl">{o.label}</p>
                <p className="val">{o.value}</p>
              </div>
              <span className="count">{linkCount(o.id)} address{linkCount(o.id) === 1 ? "" : "es"}</span>
              <CopyBtn value={o.value} label="" />
            </div>
          ))}
        </div>
      </div>

      {/* Recovery phrase */}
      <div className="pf-section">
        <div className="pf-section-h">
          <h2><Icon name="shield" size={15} /> Recovery phrase</h2>
        </div>
        <div className="pf-phrase">
          <div className="ph-top">
            <span className="t"><Icon name="lock" size={15} /> Your 24-word recovery phrase</span>
            <button className="wz-link muted" onClick={() => setRevealed((v) => !v)} style={{ fontSize: 12.5 }}>
              <Icon name={revealed ? "lock" : "eye"} size={14} />{revealed ? "Hide" : "Reveal"}
            </button>
          </div>
          {guided && (
            <p style={{ fontSize: 12.5, color: "#71717a", margin: "0 0 14px", lineHeight: 1.5 }}>
              This is the master backup for your wallet and all payout addresses. Never share it or take a screenshot.
            </p>
          )}
          <div className="wz-words-wrap">
            <div className="wz-words">
              {phrase.map((w, i) => (
                <div className="wz-word" key={i}><span className="wn">{i + 1}</span><span className="wt">{w}</span></div>
              ))}
            </div>
            {!revealed && (
              <div className="wz-blur" onClick={() => setRevealed(true)}>
                <div className="eye"><Icon name="eye" size={20} /></div>
                <div className="t">Tap to reveal</div>
                <div className="s">Make sure no one is watching</div>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Shortcuts */}
      <div className="pf-section">
        <div className="pf-tiles">
          <button className="pf-tile" onClick={() => go("dashboard")}>
            <span className="tic"><Icon name="bolt" size={20} /></span>
            <div><h3>Lightning dashboard</h3><p>Track payout status and balances</p></div>
            <span className="arr"><Icon name="arrowR" size={18} /></span>
          </button>
          <button className="pf-tile" onClick={onReturnToSetup}>
            <span className="tic"><Icon name="refresh" size={20} /></span>
            <div><h3>Re-run setup</h3><p>Return to the setup wizard</p></div>
            <span className="arr"><Icon name="arrowR" size={18} /></span>
          </button>
        </div>
      </div>
    </React.Fragment>
  );
}

Object.assign(window, { AppShell, Profile });
