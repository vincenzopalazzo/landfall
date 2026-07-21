<script lang="ts">
  import { base } from '$app/paths';
  import Nav from '$lib/components/Nav.svelte';
  import Footer from '$lib/components/Footer.svelte';
  import WaitlistForm from '$lib/components/WaitlistForm.svelte';
  import NodeWalletPreview from '$lib/components/NodeWalletPreview.svelte';
  import { waitlist } from '$lib/waitlist.svelte';
</script>

<svelte:head>
  <title>OCEAN Lightning — Early access</title>
</svelte:head>

<Nav
  links={[
    { href: '#how', label: 'How it works' },
    { href: '#payouts', label: 'Payouts' },
    { href: '#faq', label: 'FAQ' },
    { href: `${base}/docs`, label: 'Docs' }
  ]}
/>

<!-- ── HERO ── -->
<header class="hero">
  <div class="wrap">
    <div class="hero-panel">
      <div class="hero-grid">
        <div>
          <div class="hero-eyebrow"><span class="dot"></span>Early access · Limited</div>
          <h1>Your mining payouts, <em>over Lightning.</em></h1>
          <p class="hero-sub">
            Receive your OCEAN rewards straight to your own Lightning node — instant, verifiable, and
            self-custodial. Then send them anywhere. Be among the first miners to run it.
          </p>
          <WaitlistForm variant="hero" />
          <p class="hero-note">
            <span class="material-symbols-sharp">lock</span>No custody. No KYC. Runs on your node — we never
            touch your keys.
          </p>
        </div>

        <NodeWalletPreview />
      </div>
    </div>
  </div>
</header>

<!-- ── VALUE PROPS ── -->
<section class="section" id="payouts">
  <div class="wrap">
    <div class="sec-head">
      <p class="eyebrow">Why Lightning</p>
      <h2>Rewards that arrive, prove themselves, and go anywhere.</h2>
      <p>
        OCEAN Lightning turns your payouts into a wallet you control end to end — from the block OCEAN
        finds to the moment you spend it.
      </p>
    </div>
    <div class="feat-grid">
      <div class="feat">
        <div class="feat-ic"><span class="material-symbols-sharp">bolt</span></div>
        <h3>Instant, threshold-free payouts</h3>
        <p>
          Sats arrive as OCEAN finds blocks — <b>no minimum-balance thresholds, no batching delays</b>.
          Coinbase rewards mature on-chain first; your dashboard shows exactly what's spendable and what's
          still maturing.
        </p>
      </div>
      <div class="feat">
        <div class="feat-ic"><span class="material-symbols-sharp">verified</span></div>
        <h3>Every payout, provably OCEAN</h3>
        <p>
          Each reward is verified two ways — <b>paid to the BOLT12 offer you registered</b> with OCEAN, and
          carrying a <b>signed payer note</b>. Genuine payouts are flagged; anything else reads as a generic
          payment.
        </p>
      </div>
      <div class="feat">
        <div class="feat-ic"><span class="material-symbols-sharp">north_east</span></div>
        <h3>Withdraw anywhere</h3>
        <p>
          Send out over a <b>Lightning invoice, a Lightning address, on-chain to cold storage, or straight
          to an exchange</b>. Review the destination, confirm, and keep the preimage as proof of payment.
        </p>
      </div>
      <div class="feat">
        <div class="feat-ic"><span class="material-symbols-sharp">key</span></div>
        <h3>Your node, your keys</h3>
        <p>
          Payouts land on <b>your own Lightning node</b> — self-custodial from block to withdrawal. No
          intermediary balance, no KYC, no permission to move your own money.
        </p>
      </div>
    </div>
  </div>
</section>

<!-- ── HOW IT WORKS ── -->
<section class="section section-navy" id="how">
  <div class="wrap">
    <div class="sec-head">
      <p class="eyebrow">How it works</p>
      <h2>Three steps from block to balance.</h2>
    </div>
    <div class="steps">
      <div class="step">
        <h3>Register your offer</h3>
        <p>
          Link a reusable BOLT12 offer from your node to your OCEAN account. It becomes the address every
          payout is paid to — and the thing we verify against.
        </p>
        <div class="step-line"></div>
      </div>
      <div class="step">
        <h3>Receive payouts</h3>
        <p>
          As OCEAN finds blocks, rewards stream to your node. Maturing balances are shown live, with the
          confirmation count and the anchor block that gates them.
        </p>
        <div class="step-line"></div>
      </div>
      <div class="step">
        <h3>Withdraw on your terms</h3>
        <p>
          Move sats to cold storage, an exchange, or another wallet — over Lightning or on-chain —
          whenever you want, straight from the node dashboard.
        </p>
        <div class="step-line"></div>
      </div>
    </div>
  </div>
</section>

<!-- ── MATURITY BAND ── -->
<section class="section" id="maturity">
  <div class="wrap">
    <div class="sec-head">
      <p class="eyebrow">No surprises</p>
      <h2>Honest about maturity.</h2>
      <p>
        Mined rewards live in the coinbase and can't be spent until they mature. OCEAN anchors that window
        to its most recent block — so a newly found block resets the countdown. The dashboard makes it
        plain.
      </p>
    </div>
    <div class="band">
      <div class="band-cell lead">
        <div class="k"><span class="material-symbols-sharp">hourglass_top</span> Maturity window</div>
        <div class="v">Resets on each<br />new OCEAN block</div>
      </div>
      <div class="band-cell">
        <div class="k">Confirmations</div>
        <div class="v">100</div>
      </div>
      <div class="band-cell">
        <div class="k">Typical wait</div>
        <div class="v">~16<small>h</small></div>
      </div>
      <div class="band-cell">
        <div class="k">Visibility</div>
        <div class="v">Live<small> · per payout</small></div>
      </div>
    </div>
  </div>
</section>

<!-- ── FAQ ── -->
<section class="section faq-section" id="faq">
  <div class="wrap">
    <div class="sec-head center">
      <p class="eyebrow">FAQ</p>
      <h2>Questions, answered.</h2>
    </div>
    <div class="faq">
      <details open>
        <summary>What is OCEAN Lightning?<span class="material-symbols-sharp">add</span></summary>
        <div class="ans">
          A way to receive your OCEAN mining payouts over the Lightning Network, directly to a node you run.
          Instead of waiting on on-chain batches, rewards stream to your wallet, where you can track,
          verify, and withdraw them.
        </div>
      </details>
      <details>
        <summary>Do I need to run my own node?<span class="material-symbols-sharp">add</span></summary>
        <div class="ans">
          Yes — that's the point. Payouts are self-custodial: they land on your node and your keys. During
          early access we'll share setup guides for the supported node implementations.
        </div>
      </details>
      <details>
        <summary>What is the payout maturity period?<span class="material-symbols-sharp">add</span></summary>
        <div class="ans">
          Freshly mined rewards sit in the coinbase and aren't spendable until they reach
          <code>100 confirmations</code> (~16 hours). Because OCEAN anchors the window to its most recent
          block, <b>each new block OCEAN finds resets the countdown</b> for the accruing amount. Your
          dashboard shows the maturing balance, its confirmation progress, and the anchor block at all
          times.
        </div>
      </details>
      <details>
        <summary
          >How do you verify a payout really came from OCEAN?<span class="material-symbols-sharp"
            >add</span
          ></summary
        >
        <div class="ans">
          Two signals must both hold: the payment is made to the <code>BOLT12</code> offer you registered
          with OCEAN, <b>and</b> it carries a payer note matching OCEAN's signature. Meet both and it's
          flagged as a verified payout; miss either and it's treated as a generic incoming payment.
        </div>
      </details>
      <details>
        <summary>Can I withdraw to an exchange?<span class="material-symbols-sharp">add</span></summary>
        <div class="ans">
          Yes. Send out to a Lightning invoice, a Lightning address, an on-chain Bitcoin address, or a
          supported exchange deposit — with a review-and-confirm step and proof of payment on completion.
        </div>
      </details>
      <details>
        <summary>Is any of this custodial?<span class="material-symbols-sharp">add</span></summary>
        <div class="ans">
          No. OCEAN never holds your rewards on your behalf and there's no KYC. Payouts move from the pool
          to your node; withdrawals move from your node to wherever you choose.
        </div>
      </details>
    </div>
  </div>
</section>

<!-- ── FINAL CTA ── -->
<section class="section" id="access">
  <div class="wrap">
    <div class="cta-panel">
      <h2>Get early access.</h2>
      <p>
        We're onboarding miners in small batches. Leave your email and we'll reach out with an invite and
        setup guide.
      </p>
      <WaitlistForm variant="cta" />
      <p class="cta-fine">◆ {waitlist.spotsLeft} of {waitlist.totalSpots} early-access spots remaining</p>
    </div>
  </div>
</section>

<Footer />

<style>
  /* ── Hero ── */
  .hero {
    padding: 26px 0 96px;
  }
  .hero-panel {
    position: relative;
    overflow: hidden;
    border-radius: 28px;
    background: var(--gradient-hero);
    background-color: var(--ocean-navy);
    padding: 88px 72px 84px;
    color: #fff;
  }
  .hero-panel::after {
    content: '';
    position: absolute;
    inset: 0;
    background: radial-gradient(120% 90% at 12% 20%, rgba(2, 9, 45, 0.55), transparent 60%);
    pointer-events: none;
  }
  .hero-grid {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: 1.05fr 0.95fr;
    gap: 56px;
    align-items: center;
  }
  .hero-eyebrow {
    display: inline-flex;
    align-items: center;
    gap: 9px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 3px;
    text-transform: uppercase;
    color: #fff;
    background: rgba(255, 255, 255, 0.12);
    border: 1px solid rgba(255, 255, 255, 0.22);
    border-radius: var(--radius-pill);
    padding: 7px 15px;
    margin-bottom: 26px;
  }
  .hero-eyebrow .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #06ffd2;
    box-shadow: 0 0 10px #06ffd2;
    animation: pulse 2s var(--ease-in-out) infinite;
  }
  .hero h1 {
    font-family: var(--font-display);
    font-size: 66px;
    font-weight: 800;
    line-height: 1;
    letter-spacing: -2.5px;
    margin: 0 0 24px;
  }
  .hero h1 em {
    font-style: normal;
    color: #8ff5ea;
  }
  .hero-sub {
    font-size: 19px;
    line-height: 1.55;
    color: rgba(255, 255, 255, 0.82);
    max-width: 520px;
    margin: 0 0 34px;
  }
  .hero-note {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13.5px;
    color: rgba(255, 255, 255, 0.7);
    margin-top: 18px;
  }
  .hero-note .material-symbols-sharp {
    font-size: 17px;
    color: #06ffd2;
  }

  /* ── Section shells ── */
  .section {
    padding: 104px 0;
  }
  .section-navy {
    background: var(--ocean-navy);
    color: #fff;
  }
  .faq-section {
    background: var(--bg-secondary);
    color: var(--fg-primary);
  }
  .sec-head {
    max-width: 720px;
    margin: 0 0 56px;
  }
  .sec-head.center {
    margin: 0 auto 56px;
    text-align: center;
  }
  .eyebrow {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 3px;
    text-transform: uppercase;
    color: var(--ocean-blue);
    margin: 0 0 16px;
  }
  .section-navy .eyebrow {
    color: #06ffd2;
  }
  .sec-head h2 {
    font-family: var(--font-display);
    font-size: 46px;
    font-weight: 800;
    letter-spacing: -1.5px;
    line-height: 1.08;
    margin: 0 0 18px;
  }
  .sec-head p {
    font-size: 18px;
    line-height: 1.55;
    color: var(--fg-secondary);
    margin: 0;
  }
  .section-navy .sec-head p {
    color: rgba(255, 255, 255, 0.72);
  }

  /* ── Value prop cards ── */
  .feat-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 20px;
  }
  .feat {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 16px;
    padding: 32px;
  }
  .feat:hover {
    border-color: var(--ocean-blue-light);
  }
  .feat-ic {
    width: 46px;
    height: 46px;
    border-radius: 12px;
    background: var(--ocean-blue-subtle);
    color: var(--ocean-blue);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 22px;
  }
  .feat-ic .material-symbols-sharp {
    font-size: 24px;
  }
  .feat h3 {
    font-family: var(--font-display);
    font-size: 21px;
    font-weight: 700;
    letter-spacing: -0.4px;
    margin: 0 0 10px;
  }
  .feat p {
    font-size: 15.5px;
    line-height: 1.6;
    color: var(--fg-secondary);
    margin: 0;
  }
  .feat p b {
    color: var(--fg-primary);
    font-weight: 600;
  }

  /* ── How it works ── */
  .steps {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 24px;
    counter-reset: step;
  }
  .step {
    position: relative;
    padding-top: 30px;
  }
  .step::before {
    counter-increment: step;
    content: counter(step, decimal-leading-zero);
    font-family: var(--font-mono);
    font-size: 13px;
    font-weight: 600;
    color: #06ffd2;
    letter-spacing: 0.04em;
  }
  .step h3 {
    font-family: var(--font-display);
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.5px;
    margin: 14px 0 10px;
  }
  .step p {
    font-size: 15.5px;
    line-height: 1.6;
    color: rgba(255, 255, 255, 0.72);
    margin: 0;
  }
  .step-line {
    height: 1px;
    background: rgba(255, 255, 255, 0.14);
    margin-top: 30px;
  }

  /* ── Maturity stat band ── */
  .band {
    display: grid;
    grid-template-columns: 1.2fr 1fr 1fr 1fr;
    gap: 0;
    border: 1px solid var(--border);
    border-radius: 16px;
    overflow: hidden;
    background: var(--bg-secondary);
  }
  .band-cell {
    padding: 28px 30px;
    border-right: 1px solid var(--border);
  }
  .band-cell:last-child {
    border-right: 0;
  }
  .band-cell .k {
    font-size: 13px;
    color: var(--fg-tertiary);
    margin-bottom: 10px;
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .band-cell .k .material-symbols-sharp {
    font-size: 16px;
    color: var(--ocean-blue);
  }
  .band-cell .v {
    font-family: var(--font-mono);
    font-size: 26px;
    font-weight: 500;
    letter-spacing: -0.02em;
    color: var(--fg-primary);
  }
  .band-cell .v small {
    font-size: 14px;
    color: var(--fg-tertiary);
  }
  .band-cell.lead .v {
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.4px;
    line-height: 1.25;
  }

  /* ── FAQ ── */
  .faq {
    max-width: 820px;
    margin: 0 auto;
  }
  .faq details {
    border-bottom: 1px solid var(--border);
    padding: 4px 0;
  }
  .faq summary {
    list-style: none;
    cursor: pointer;
    padding: 22px 0;
    display: flex;
    align-items: center;
    gap: 16px;
    font-family: var(--font-display);
    font-size: 19px;
    font-weight: 600;
    letter-spacing: -0.3px;
    color: var(--fg-primary);
  }
  .faq summary::-webkit-details-marker {
    display: none;
  }
  .faq summary .material-symbols-sharp {
    margin-left: auto;
    color: var(--ocean-blue);
    transition: transform var(--dur-base) var(--ease-out);
    font-size: 24px;
  }
  .faq details[open] summary .material-symbols-sharp {
    transform: rotate(45deg);
  }
  .faq .ans {
    padding: 0 40px 24px 0;
    font-size: 16px;
    line-height: 1.65;
    color: var(--fg-secondary);
  }
  .faq .ans code {
    font-family: var(--font-mono);
    font-size: 14px;
    background: var(--bg-tertiary);
    border-radius: 6px;
    padding: 1px 6px;
    color: var(--fg-primary);
  }

  /* ── Final CTA ── */
  .cta-panel {
    background: var(--gradient-cta);
    border-radius: 28px;
    padding: 72px 64px;
    text-align: center;
    color: var(--ocean-navy);
  }
  .cta-panel h2 {
    font-family: var(--font-display);
    font-size: 48px;
    font-weight: 800;
    letter-spacing: -1.8px;
    line-height: 1.02;
    margin: 0 0 18px;
  }
  .cta-panel p {
    font-size: 18px;
    line-height: 1.5;
    color: rgba(2, 9, 45, 0.7);
    max-width: 520px;
    margin: 0 auto 34px;
  }
  .cta-fine {
    font-family: var(--font-mono);
    font-size: 12px;
    color: rgba(2, 9, 45, 0.55);
    margin-top: 20px;
    letter-spacing: 0.02em;
  }

  @media (max-width: 940px) {
    .hero-grid {
      grid-template-columns: 1fr;
      gap: 44px;
    }
    .hero h1 {
      font-size: 52px;
    }
    .feat-grid,
    .steps,
    .band {
      grid-template-columns: 1fr;
    }
    .band-cell {
      border-right: 0;
      border-bottom: 1px solid var(--border);
    }
  }
  @media (max-width: 560px) {
    .hero-panel {
      padding: 52px 28px 48px;
      border-radius: 20px;
    }
    .hero h1 {
      font-size: 40px;
      letter-spacing: -1.5px;
    }
    .section {
      padding: 72px 0;
    }
    .cta-panel {
      padding: 48px 26px;
    }
    .cta-panel h2,
    .sec-head h2 {
      font-size: 34px;
    }
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }
</style>
