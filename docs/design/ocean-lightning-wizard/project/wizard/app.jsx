// wizard/app.jsx — window chrome, step rail, navigation, state, tweaks.
const { useState, useEffect } = React;

const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "density": "guided",
  "accent": "blue",
  "phraseLen": 24
}/*EDITMODE-END*/;

const ACCENTS = {
  orange: { c: "#f7931a", dim: "rgba(247,147,26,0.10)", line: "rgba(247,147,26,0.28)" },
  blue:   { c: "#4D6BFF", dim: "rgba(77,107,255,0.12)", line: "rgba(77,107,255,0.32)" },
};

// confirm-quiz positions must match ConfirmStep
const confirmPicks = (len) => (len >= 24 ? [3, 12, 20] : [2, 6, 10]);

// Seed the profile (onchain addresses + offers) from the setup results.
const seedProfile = (desc) => ({
  offers: [{ id: "o1", label: desc || "OCEAN mining payouts", value: ARTIFACTS.offer }],
  addresses: [{ id: "a1", label: "Primary payout", address: ARTIFACTS.address, offerId: "o1" }],
});

function App() {
  const [t, setTweak] = useTweaks(TWEAK_DEFAULTS);
  const guided = t.density === "guided";
  const phrase = PHRASE_24.slice(0, t.phraseLen);

  const [stepIndex, setStepIndex] = useState(0);
  const [surface, setSurface] = useState("wizard"); // 'wizard' | 'profile' | 'dashboard'
  const [mode, setMode] = useState("create");
  const [offerDescription, setOfferDescription] = useState("");
  const [profile, setProfile] = useState(null);
  const [revealed, setRevealed] = useState(false);
  const [backedUp, setBackedUp] = useState(false);
  const [importWords, setImportWords] = useState(() => Array(t.phraseLen).fill(""));
  const [answers, setAnswers] = useState({});
  const [provisionDone, setProvisionDone] = useState(false);
  const [signed, setSigned] = useState(false);
  const [verifyState, setVerifyState] = useState("idle");

  // apply accent globally
  useEffect(() => {
    const a = ACCENTS[t.accent] || ACCENTS.blue;
    const r = document.documentElement.style;
    r.setProperty("--wiz-accent", a.c);
    r.setProperty("--wiz-accent-dim", a.dim);
    r.setProperty("--wiz-accent-line", a.line);
  }, [t.accent]);

  // reset phrase-dependent state if length changes
  useEffect(() => {
    setImportWords(Array(t.phraseLen).fill(""));
    setAnswers({});
  }, [t.phraseLen]);

  const isImport = mode === "import";
  const skipConfirm = isImport; // imported phrases don't need re-confirmation

  // ── gating ──
  const importFilled = importWords.every((w) => w && w.length > 1);
  const confirmOK = confirmPicks(t.phraseLen).every((idx, qi) => answers[qi] === phrase[idx]);
  const canContinue = (() => {
    switch (STEPS[stepIndex].key) {
      case "phrase":  return isImport ? importFilled : (revealed && backedUp);
      case "confirm": return confirmOK;
      case "wallet":  return provisionDone;
      case "sign":    return signed;
      default:        return true;
    }
  })();

  const goNext = () => {
    let n = stepIndex + 1;
    if (STEPS[n] && STEPS[n].key === "confirm" && skipConfirm) n += 1;
    setStepIndex(Math.min(n, STEPS.length - 1));
  };
  const goBack = () => {
    let n = stepIndex - 1;
    if (STEPS[n] && STEPS[n].key === "confirm" && skipConfirm) n -= 1;
    setStepIndex(Math.max(n, 0));
  };
  const choose = (m) => { setMode(m); setStepIndex(1); };
  const restart = () => {
    setStepIndex(0); setMode("create"); setRevealed(false); setBackedUp(false);
    setImportWords(Array(t.phraseLen).fill("")); setAnswers({});
    setProvisionDone(false); setSigned(false); setVerifyState("idle");
    setSurface("wizard"); setOfferDescription(""); setProfile(null);
  };

  // navigate post-setup surfaces (seed profile on first visit)
  const go = (s) => {
    if (s === "profile" && !profile) setProfile(seedProfile(offerDescription));
    setSurface(s);
  };

  const railClick = (i) => { if (i < stepIndex) { if (STEPS[i].key === "confirm" && skipConfirm) return; setStepIndex(i); } };

  const key = STEPS[stepIndex].key;
  const stepEl = (() => {
    switch (key) {
      case "welcome": return <WelcomeStep guided={guided} onChoose={choose} />;
      case "phrase":  return <PhraseStep guided={guided} mode={mode} phrase={phrase} revealed={revealed} setRevealed={setRevealed} backedUp={backedUp} setBackedUp={setBackedUp} importWords={importWords} setImportWords={setImportWords} />;
      case "confirm": return <ConfirmStep key={t.phraseLen} guided={guided} phrase={phrase} answers={answers} setAnswers={setAnswers} />;
      case "wallet":  return <WalletStep guided={guided} provisionDone={provisionDone} setProvisionDone={setProvisionDone} offerDescription={offerDescription} setOfferDescription={setOfferDescription} />;
      case "sign":    return <SignStep guided={guided} signed={signed} setSigned={setSigned} />;
      case "done":    return <DoneStep guided={guided} verifyState={verifyState} setVerifyState={setVerifyState} onFinish={() => go("profile")} />;
    }
  })();

  const visibleSteps = STEPS.length - (skipConfirm ? 1 : 0);
  const humanIndex = stepIndex - (skipConfirm && stepIndex > 2 ? 1 : 0) + 1;

  return (
    <React.Fragment>
      <div className="wz-window">
        <div className="wz-body">
          {surface === "profile" || surface === "dashboard" ? (
            <AppShell active={surface} go={go}>
              {surface === "profile"
                ? <Profile guided={guided} phrase={phrase} profile={profile || seedProfile(offerDescription)} setProfile={setProfile} go={go} onReturnToSetup={() => setSurface("wizard")} />
                : <Dashboard guided={guided} offerDescription={offerDescription} />}
            </AppShell>
          ) : (
          <React.Fragment>
          {/* rail */}
          <div className="wz-rail">
            <div className="wz-rail-brand">
              <img src="assets/OCEAN-logo-white.svg" alt="OCEAN" />
              <span className="tag">Lightning</span>
            </div>
            {STEPS.map((s, i) => {
              const skipped = s.key === "confirm" && skipConfirm;
              const state = skipped ? "skip" : i < stepIndex ? "done" : i === stepIndex ? "active" : "";
              return (
                <div key={s.key} className={`wz-step ${state === "skip" ? "" : state}`}
                  style={{ cursor: i < stepIndex && !skipped ? "pointer" : "default", opacity: skipped ? 0.32 : 1 }}
                  onClick={() => railClick(i)}>
                  <div className="num">
                    {state === "done" ? <Icon name="check" size={13} stroke={2} />
                      : skipped ? "–" : i + 1}
                  </div>
                  <div className="lbl">{s.label}</div>
                </div>
              );
            })}
            <div className="wz-rail-foot">
              <div className="note">
                <Icon name="lock" size={13} />
                <span>Your keys and recovery phrase stay on your device.</span>
              </div>
            </div>
          </div>

          {/* content + footer */}
          <div className="wz-content">
            <div className="wz-scroll" key={key}>{stepEl}</div>
            <div className="wz-foot">
              {stepIndex > 0
                ? <Btn variant="ghost" icon="arrowL" onClick={goBack}>Back</Btn>
                : <span />}
              <span className="spacer" />
              <span className="wz-progress-text">Step {Math.min(humanIndex, visibleSteps)} of {visibleSteps}</span>
              {key === "welcome" && <span style={{ width: 4 }} />}
              {key === "done"
                ? (verifyState === "verified"
                    ? <Btn icon="refresh" onClick={restart}>Start over</Btn>
                    : null)
                : key !== "welcome"
                  ? <Btn iconRight="arrowR" disabled={!canContinue} onClick={goNext}>Continue</Btn>
                  : null}
            </div>
          </div>
          </React.Fragment>
          )}
        </div>
      </div>

      {/* Tweaks */}
      <TweaksPanel>
        <TweakSection label="Guidance" />
        <TweakRadio label="Explainers" value={t.density} options={["guided", "concise"]} onChange={(v) => setTweak("density", v)} />
        <TweakSection label="Wallet" />
        <TweakRadio label="Phrase length" value={t.phraseLen} options={[24, 12]} onChange={(v) => setTweak("phraseLen", v)} />
      </TweaksPanel>
    </React.Fragment>
  );
}

window.App = App;
