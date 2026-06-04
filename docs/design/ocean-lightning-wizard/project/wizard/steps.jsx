// wizard/steps.jsx — the six wizard screens.
const { useState, useEffect, useRef, useMemo } = React;

// ───────────────────────────── 1. WELCOME ─────────────────────────────
function WelcomeStep({ guided, onChoose }) {
  return (
    <div className="wz-fade">
      <p className="wz-eyebrow"><Icon name="bolt" size={13} /> Lightning payouts</p>
      <h1 className="wz-h">Get paid your mining rewards over Lightning</h1>
      <p className="wz-sub">
        Instead of waiting for on-chain payouts, OCEAN can send your rewards to a
        Lightning wallet you fully control. This setup takes about three minutes —
        we'll walk you through every step.
      </p>

      <div className="wz-card" style={{ marginBottom: 22, display: "flex", gap: 16, alignItems: "flex-start" }}>
        <div style={{ width: 38, height: 38, borderRadius: 10, flexShrink: 0, background: "var(--wiz-accent-dim)", color: "var(--wiz-accent)", display: "flex", alignItems: "center", justifyContent: "center" }}>
          <Icon name="key" size={19} />
        </div>
        <div>
          <div style={{ fontSize: 14, fontWeight: 600, color: "#fafafa", marginBottom: 4, display: "flex", alignItems: "center", gap: 8 }}>
            One recovery phrase runs everything
            <Tip enabled={guided}>
              <b>Your recovery phrase</b> is a list of words that is the master key to
              your money. Here, the same phrase powers both your Lightning wallet and
              the Bitcoin address OCEAN pays you at — so there's only one thing to back up.
            </Tip>
          </div>
          <p style={{ fontSize: 13, lineHeight: 1.6, color: "#a1a1aa", margin: 0 }}>
            A single set of backup words controls your Lightning wallet <em>and</em> your
            payout address. Back it up once and you're covered for both.
          </p>
        </div>
      </div>

      <Callout kind="info" icon="shield">
        <b>You stay in control.</b> Your wallet runs in a private, sealed environment
        (powered by Lexe) that only you can open. OCEAN never holds your keys or your funds —
        it only learns where to send your rewards.
      </Callout>

      <p className="wz-section-label" style={{ marginTop: 26 }}>How would you like to start?</p>
      <div className="wz-choices">
        <button className="wz-choice" onClick={() => onChoose("create")}>
          <span className="rec">Recommended</span>
          <span className="ic"><Icon name="spark" size={20} /></span>
          <h4>Create a new wallet</h4>
          <p>We'll generate a fresh recovery phrase and set everything up for you.</p>
        </button>
        <button className="wz-choice" onClick={() => onChoose("import")}>
          <span className="ic"><Icon name="download" size={20} /></span>
          <h4>I already have a phrase</h4>
          <p>Restore an existing 12 or 24-word recovery phrase you've used before.</p>
        </button>
      </div>
    </div>
  );
}

// ───────────────────────────── 2. PHRASE ─────────────────────────────
function PhraseStep({ guided, mode, phrase, revealed, setRevealed, backedUp, setBackedUp, importWords, setImportWords }) {
  if (mode === "import") {
    const fill = (i, v) => {
      const next = importWords.slice();
      next[i] = v.trim().toLowerCase();
      setImportWords(next);
    };
    const onPaste = (e) => {
      const parts = e.clipboardData.getData("text").trim().split(/\s+/);
      if (parts.length > 1) {
        e.preventDefault();
        const next = importWords.slice();
        parts.slice(0, next.length).forEach((w, i) => { next[i] = w.toLowerCase(); });
        setImportWords(next);
      }
    };
    return (
      <div className="wz-fade">
        <p className="wz-eyebrow"><Icon name="download" size={13} /> Restore wallet</p>
        <h1 className="wz-h">Enter your recovery phrase</h1>
        <p className="wz-sub">
          Type each word in order, or paste the whole phrase into the first box.
          Words are never sent anywhere — everything happens on your device.
        </p>
        <Callout kind="warn" icon="warn">
          Only enter your phrase on a device you trust. OCEAN staff will <b>never</b> ask
          you for these words.
        </Callout>
        <div className="wz-import">
          {importWords.map((w, i) => (
            <label className="ipt" key={i}>
              <span className="n">{i + 1}</span>
              <input value={w} onChange={(e) => fill(i, e.target.value)} onPaste={i === 0 ? onPaste : undefined}
                autoCapitalize="off" autoCorrect="off" spellCheck="false" />
            </label>
          ))}
        </div>
      </div>
    );
  }

  // create mode — reveal + back up
  return (
    <div className="wz-fade">
      <p className="wz-eyebrow"><Icon name="key" size={13} /> Your recovery phrase</p>
      <h1 className="wz-h">Write down these {phrase.length} words</h1>
      <p className="wz-sub">
        This is the <strong>only</strong> backup of your wallet. Write the words on paper
        in this exact order and keep them somewhere safe and private. Whoever has these
        words controls the funds.
      </p>

      <div className="wz-words-wrap" style={{ marginBottom: 18 }}>
        <div className={`wz-words ${phrase.length === 12 ? "cols3" : ""}`}>
          {phrase.map((w, i) => (
            <div className="wz-word" key={i}>
              <span className="wn">{i + 1}</span>
              <span className="wt">{w}</span>
            </div>
          ))}
        </div>
        {!revealed && (
          <div className="wz-blur" onClick={() => setRevealed(true)}>
            <div className="eye"><Icon name="eye" size={20} /></div>
            <div className="t">Tap to reveal your phrase</div>
            <div className="s">Make sure no one is looking over your shoulder</div>
          </div>
        )}
      </div>

      {revealed && (
        <>
          <Callout kind="danger" icon="warn">
            <b>Never share these words and never take a screenshot.</b> Anyone who sees them
            can take your Bitcoin. No real service will ever ask for them.
          </Callout>
          <label style={{ display: "flex", gap: 11, alignItems: "flex-start", cursor: "pointer", padding: "4px 2px" }}>
            <input type="checkbox" checked={backedUp} onChange={(e) => setBackedUp(e.target.checked)}
              style={{ width: 18, height: 18, marginTop: 1, accentColor: "var(--wiz-accent)", flexShrink: 0, cursor: "pointer" }} />
            <span style={{ fontSize: 14, color: "#d4d4d8", lineHeight: 1.5 }}>
              I've written my recovery phrase down and stored it somewhere safe.
            </span>
          </label>
          {guided && (
            <p style={{ fontSize: 12, color: "#52525b", margin: "14px 0 0", display: "flex", alignItems: "center", gap: 7 }}>
              <Icon name="lock" size={13} /> We don't keep a copy — there's no "forgot password" for this.
            </p>
          )}
        </>
      )}
    </div>
  );
}

// ───────────────────────────── 3. CONFIRM ─────────────────────────────
function ConfirmStep({ guided, phrase, answers, setAnswers }) {
  // Pick 3 word positions + build stable multiple-choice options.
  const quiz = useRef(null);
  if (!quiz.current) {
    const n = phrase.length;
    const picks = n >= 24 ? [3, 12, 20] : [2, 6, 10]; // 0-indexed
    const decoys = WORD_POOL.filter((w) => !phrase.includes(w));
    quiz.current = picks.map((idx) => {
      const correct = phrase[idx];
      const opts = [correct];
      while (opts.length < 4) {
        const d = decoys[Math.floor(Math.random() * decoys.length)];
        if (!opts.includes(d)) opts.push(d);
      }
      for (let i = opts.length - 1; i > 0; i--) { const j = Math.floor(Math.random() * (i + 1)); [opts[i], opts[j]] = [opts[j], opts[i]]; }
      return { idx, correct, opts };
    });
  }
  const pick = (qi, word) => {
    setAnswers((prev) => ({ ...prev, [qi]: word }));
  };
  return (
    <div className="wz-fade">
      <p className="wz-eyebrow"><Icon name="check" size={13} /> Quick check</p>
      <h1 className="wz-h">Confirm your backup</h1>
      <p className="wz-sub">
        Let's make sure your backup is correct. Using the words you just wrote down,
        tap the right word for each position below.
      </p>
      {quiz.current.map((q, qi) => {
        const chosen = answers[qi];
        return (
          <div className="wz-confirm-q" key={qi}>
            <p className="q">Word <b>#{q.idx + 1}</b></p>
            <div className="wz-opts">
              {q.opts.map((w) => {
                let cls = "wz-opt";
                if (chosen === w) cls += w === q.correct ? " correct" : " wrong";
                else if (chosen && w === q.correct) cls += "";
                return (
                  <button key={w} className={cls} onClick={() => pick(qi, w)} type="button">{w}</button>
                );
              })}
            </div>
            {chosen && chosen !== q.correct && (
              <p style={{ fontSize: 12, color: "#ef4444", margin: "8px 0 0" }}>That's not the right word — check your written backup and try again.</p>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ───────────────────────────── 4. WALLET (description → offer → provision) ─────────────────────────────
function WalletStep({ guided, provisionDone, setProvisionDone, offerDescription, setOfferDescription }) {
  const [phase, setPhase] = useState(provisionDone ? "done" : "input");
  const [taskIndex, setTaskIndex] = useState(provisionDone ? PROVISION_TASKS.length : 0);

  const start = () => {
    setPhase("running");
    const durations = [1000, 1200, 1000];
    let i = 0;
    const run = () => {
      if (i >= PROVISION_TASKS.length) { setProvisionDone(true); setPhase("done"); return; }
      setTaskIndex(i);
      setTimeout(() => { i += 1; setTaskIndex(i); run(); }, durations[i] || 1000);
    };
    run();
  };

  // ── input phase: collect the offer description ──
  if (phase === "input") {
    return (
      <div className="wz-fade">
        <p className="wz-eyebrow"><Icon name="wallet" size={13} /> Create your wallet</p>
        <h1 className="wz-h">Describe your Lightning offer</h1>
        <p className="wz-sub">
          Add a short description so you (and OCEAN) can recognize this payout destination.
          It's saved inside your <strong>BOLT12 offer</strong> and shown on every payment.
        </p>
        <div className="wz-field">
          <label>
            Offer description
            <Tip enabled={guided}>
              <span>This label is encoded into your <b>BOLT12 offer</b>. It travels with the
              offer so any payment to it carries this note — handy for bookkeeping.</span>
            </Tip>
          </label>
          <input className="wz-input" value={offerDescription} maxLength={64}
            placeholder="e.g. OCEAN mining payouts"
            onChange={(e) => setOfferDescription(e.target.value)} />
          <div className="wz-suggest">
            {OFFER_SUGGESTIONS.map((s) => (
              <button key={s} className="wz-chip-btn" type="button" onClick={() => setOfferDescription(s)}>{s}</button>
            ))}
          </div>
        </div>
        <Callout kind="info" icon="enclave">
          When you continue, we start your node in a sealed enclave and generate your
          offer and payout address — this happens once and takes a few seconds.
        </Callout>
        <Btn icon="spark" disabled={!offerDescription.trim()} onClick={start}>Create wallet &amp; offer</Btn>
      </div>
    );
  }

  const allDone = phase === "done";
  return (
    <div className="wz-fade">
      <p className="wz-eyebrow"><Icon name="wallet" size={13} /> {allDone ? "Wallet ready" : "Creating your wallet"}</p>
      <h1 className="wz-h">{allDone ? "Your wallet is ready" : "Setting up your Lightning wallet"}</h1>
      <p className="wz-sub">
        {allDone
          ? "Your Lightning wallet is live and we've generated your payout details from your recovery phrase."
          : "Hang tight — this only happens once. We're starting your node and building your offer and payout address."}
      </p>

      <div className="wz-tasks">
        {PROVISION_TASKS.map((tk, i) => {
          const state = i < taskIndex || allDone ? "done" : i === taskIndex ? "active" : "";
          return (
            <div className={`wz-task ${state}`} key={i}>
              <div className="tk-ic">
                {state === "done" ? <Icon name="check" size={14} />
                  : state === "active" ? <span className="wz-spinner" />
                  : <span style={{ fontFamily: "var(--font-mono)", fontSize: 11 }}>{i + 1}</span>}
              </div>
              <div>
                <div className="tk-t">{tk.t}</div>
                {(state === "active" || (guided && state === "done")) && <div className="tk-s">{tk.s}</div>}
              </div>
            </div>
          );
        })}
      </div>

      {allDone && (
        <div className="wz-fade" style={{ marginTop: 22 }}>
          <Callout kind="ok" icon="enclave">
            <b>Running in a sealed enclave.</b> Your node lives in hardware only you can
            unlock. Even Lexe can't see your keys or move your funds.
          </Callout>
          <CopyField label="Your Lightning address (offer)" chip="BOLT12" value={ARTIFACTS.offer}
            tip={<span><b>A BOLT12 offer</b> is a reusable Lightning address (it starts with <b>lno1</b>). OCEAN sends your rewards to it — you can reuse it forever.</span>} />
          {offerDescription.trim() && (
            <div className="wz-offer-desc"><Icon name="info" size={14} /> Description encoded in offer: <b>{offerDescription}</b></div>
          )}
          <CopyField label="Your payout address" chip="bc1q" value={ARTIFACTS.address}
            tip={<span>A standard Bitcoin address derived from your phrase. OCEAN uses it as your verified payout identity.</span>}
            />
        </div>
      )}
    </div>
  );
}

// ───────────────────────────── 5. SIGN ─────────────────────────────
function SignStep({ guided, signed, setSigned }) {
  const [signing, setSigning] = useState(false);
  const sign = () => {
    setSigning(true);
    setTimeout(() => { setSigning(false); setSigned(true); }, 1300);
  };
  return (
    <div className="wz-fade">
      <p className="wz-eyebrow"><Icon name="pen" size={13} /> Prove it's you</p>
      <h1 className="wz-h">Sign OCEAN's verification message</h1>
      <p className="wz-sub">
        To switch on payouts, OCEAN needs proof you control your payout address. You'll
        sign a short message with your wallet — like a digital signature. {guided && "It costs nothing and moves no money."}
      </p>

      <p className="wz-section-label">Message from OCEAN
        <Tip enabled={guided} label=" what's a signature?">
          <span>A <b>BIP-322 signature</b> proves you hold the private key for your address
          without revealing it. It's math, not a password — and it can't be used to spend your funds.</span>
        </Tip>
      </p>
      <div className="wz-copy" style={{ marginBottom: 18 }}>
        <div className="val" style={{ whiteSpace: "pre-wrap", color: "#a1a1aa" }}>{OCEAN_MESSAGE}</div>
      </div>

      {!signed ? (
        <div className="wz-verify-row">
          {signing ? (
            <div className="wz-verifying"><span className="wz-spinner" /> Signing with your wallet…</div>
          ) : (
            <Btn icon="pen" onClick={sign}>Sign message</Btn>
          )}
        </div>
      ) : (
        <div className="wz-fade">
          <Callout kind="ok" icon="check"><b>Signed.</b> Here's your signature — OCEAN will check it against your payout address.</Callout>
          <CopyField label="Your signature" chip="BIP-322" value={ARTIFACTS.signature}
            tip={<span>This string is the proof. OCEAN verifies it matches your address; it can't be reused to access your wallet.</span>} />
        </div>
      )}
    </div>
  );
}

// ───────────────────────────── 6. DONE / HAND-OFF ─────────────────────────────
function DoneStep({ guided, verifyState, setVerifyState, onFinish }) {
  const verify = () => {
    setVerifyState("verifying");
    setTimeout(() => setVerifyState("verified"), 1800);
  };

  if (verifyState === "verified") {
    return (
      <div className="wz-fade">
        <div className="wz-seal"><Icon name="check" size={34} stroke={2} /></div>
        <p className="wz-eyebrow" style={{ color: "#22c55e" }}><Icon name="bolt" size={13} /> Payouts enabled</p>
        <h1 className="wz-h">Lightning payouts are on</h1>
        <p className="wz-sub">
          You're all set. From your next payout onward, OCEAN will send your mining rewards
          straight to your Lightning wallet.
        </p>
        <div className="wz-summary">
          <div className="row"><span className="ri"><Icon name="wallet" size={17} /></span><span className="rk">Lightning offer</span><span className="rv">{ARTIFACTS.offer}</span></div>
          <div className="row"><span className="ri"><Icon name="key" size={17} /></span><span className="rk">Payout address</span><span className="rv">{ARTIFACTS.address}</span></div>
          <div className="row"><span className="ri"><Icon name="shield" size={17} /></span><span className="rk">Self-custody</span><span className="rv">You hold the keys</span></div>
        </div>
        <Callout kind="info" icon="info" style={{ marginTop: 18 }}>
          Keep your recovery phrase safe. It's the only way to restore this wallet and your payouts.
        </Callout>
        <div style={{ marginTop: 22 }}>
          <Btn icon="wallet" onClick={onFinish}>Go to my profile</Btn>
        </div>
      </div>
    );
  }

  return (
    <div className="wz-fade">
      <p className="wz-eyebrow"><Icon name="link" size={13} /> Final step</p>
      <h1 className="wz-h">Send your details to OCEAN</h1>
      <p className="wz-sub">
        Here are the three things OCEAN needs. Submit them now, or copy each one into
        OCEAN's payout settings yourself — whichever you prefer.
      </p>

      <CopyField label="Payout address" chip="bc1q" value={ARTIFACTS.address}
        tip={guided ? <span>Where OCEAN records your rewards as paid.</span> : null} />
      <CopyField label="Lightning offer" chip="BOLT12" value={ARTIFACTS.offer}
        tip={guided ? <span>The reusable Lightning address rewards are sent to.</span> : null} />
      <CopyField label="Signature" chip="BIP-322" value={ARTIFACTS.signature}
        tip={guided ? <span>Proof you control the payout address.</span> : null} />

      <div className="wz-verify-row">
        {verifyState === "verifying" ? (
          <div className="wz-verifying"><span className="wz-spinner" /> Verifying with OCEAN…</div>
        ) : (
          <>
            <Btn icon="bolt" onClick={verify}>Verify &amp; turn on payouts</Btn>
            <span style={{ fontSize: 12.5, color: "#52525b" }}>or copy the fields above into OCEAN manually</span>
          </>
        )}
      </div>
    </div>
  );
}

Object.assign(window, { WelcomeStep, PhraseStep, ConfirmStep, WalletStep, SignStep, DoneStep });
