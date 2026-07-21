<script lang="ts">
  import { base } from '$app/paths';
</script>

<footer class="footer">
  <div class="wrap footer-in">
    <div>
      <img class="footer-logo" src="{base}/assets/OCEAN-logo-white.svg" alt="OCEAN" />
      <p class="tag">A new type of mining pool on a mission to radically decentralize Bitcoin mining.</p>
    </div>
    <div class="footer-cols">
      <div class="footer-col">
        <h4>Product</h4>
        <a href="{base}/#payouts">Lightning payouts</a>
        <a href="{base}/#how">How it works</a>
        <a href="{base}/#maturity">Maturity</a>
        <a href="{base}/#faq">FAQ</a>
      </div>
      <div class="footer-col">
        <h4>Docs</h4>
        <a href="{base}/docs/getting-started">Getting started</a>
        <a href="{base}/docs/cli">CLI reference</a>
        <a href="{base}/docs/http-api">HTTP API</a>
        <a href="{base}/docs/wizard">Web wizard</a>
      </div>
      <div class="footer-col">
        <h4>OCEAN</h4>
        <a href="https://ocean.xyz" target="_blank" rel="noreferrer noopener">Pool stats &#8599;</a>
        <a href="https://ocean.xyz" target="_blank" rel="noreferrer noopener">Blocks found &#8599;</a>
        <a href="https://ocean.xyz" target="_blank" rel="noreferrer noopener">DATUM &#8599;</a>
      </div>
    </div>
  </div>
  <div class="footer-bar">
    <span>© 2026 OCEAN. All rights reserved.</span>
    <div class="footer-social">
      <a href="https://x.com/ocean_mining" aria-label="X" target="_blank" rel="noreferrer noopener"
        ><img src="{base}/assets/symbol.svg" alt="" /></a
      >
    </div>
  </div>
</footer>

<style>
  .footer {
    background: var(--ocean-navy);
    color: rgba(255, 255, 255, 0.6);
    padding: 56px 0 40px;
  }
  .footer-in {
    display: flex;
    align-items: flex-start;
    gap: 40px;
    flex-wrap: wrap;
  }
  .footer-logo {
    height: 24px;
    opacity: 0.95;
  }
  .footer .tag {
    font-size: 14px;
    line-height: 1.6;
    max-width: 280px;
    margin: 18px 0 0;
  }
  .footer-cols {
    margin-left: auto;
    display: flex;
    gap: 64px;
  }
  .footer-col h4 {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.4);
    margin: 0 0 16px;
  }
  .footer-col a {
    display: block;
    color: rgba(255, 255, 255, 0.72);
    font-size: 14px;
    margin-bottom: 11px;
  }
  .footer-col a:hover {
    color: #fff;
  }
  .footer-bar {
    max-width: 1200px;
    margin: 44px auto 0;
    padding: 22px 40px 0;
    border-top: 1px solid rgba(255, 255, 255, 0.12);
    display: flex;
    align-items: center;
    gap: 16px;
    font-size: 13px;
    color: rgba(255, 255, 255, 0.5);
  }
  .footer-social {
    margin-left: auto;
    display: flex;
    gap: 14px;
  }
  .footer-social a {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    border: 1px solid rgba(255, 255, 255, 0.16);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .footer-social a:hover {
    border-color: rgba(255, 255, 255, 0.4);
    background: rgba(255, 255, 255, 0.05);
  }
  .footer-social img {
    width: 15px;
    height: 15px;
  }

  @media (max-width: 940px) {
    .footer-cols {
      margin-left: 0;
      width: 100%;
      gap: 40px;
    }
  }
  @media (max-width: 560px) {
    .footer-bar {
      padding: 22px 22px 0;
    }
  }
</style>
