<script lang="ts">
  import { base } from '$app/paths';
</script>

<!-- Product preview: mirrors the real node wallet. Decorative. -->
<div class="pv" aria-hidden="true">
  <div class="pv-top">
    <span class="t">Node wallet</span>
    <span class="pv-live"><span class="d"></span>Node online</span>
  </div>
  <div class="pv-bal">
    <div class="l">Lightning channel · spendable</div>
    <div class="v">1,284,503 <span class="u">sats</span></div>
    <div class="f">$1,235.09</div>
  </div>
  <div class="pv-row ocean">
    <span class="pv-ic"><img src="{base}/assets/OCEAN-icon-white.svg" alt="" /></span>
    <span class="pv-meta">
      <span class="pv-t"
        >OCEAN <span class="pv-chip"><img src="{base}/assets/OCEAN-icon-white.svg" alt="" />Payout</span></span
      >
      <span class="pv-s"><span>Lightning</span><span class="sdot"></span><span>9m ago</span></span>
    </span>
    <span class="pv-amt"><span class="v in">+12,084<span class="u"> sats</span></span><span class="f">$11.63</span></span>
  </div>
  <div class="pv-row ocean">
    <span class="pv-ic" style="background:rgba(255,179,0,0.10);border-color:rgba(255,179,0,0.3)"
      ><span class="material-symbols-sharp">hourglass_top</span></span
    >
    <span class="pv-meta">
      <span class="pv-t"
        >OCEAN <span class="pv-chip"><img src="{base}/assets/OCEAN-icon-white.svg" alt="" />Payout</span></span
      >
      <span class="pv-s"
        ><span>Matures ~15h 40m</span><span class="sdot"></span><span>block 897,180</span></span
      >
    </span>
    <span class="pv-amt"
      ><span class="v mat">47,820<span class="u"> sats</span></span><span class="f">maturing</span></span
    >
  </div>
  <div class="pv-row">
    <span class="pv-ic"
      ><span class="material-symbols-sharp" style="color:#a1a1aa">south_west</span></span
    >
    <span class="pv-meta">
      <span class="pv-t" style="color:#fafafa">satoshi@wallet…</span>
      <span class="pv-s"><span>Lightning</span><span class="sdot"></span><span>Tip</span></span>
    </span>
    <span class="pv-amt"><span class="v in">+25,000<span class="u"> sats</span></span><span class="f">$24.05</span></span>
  </div>
</div>

<style>
  .pv {
    background: #0b0b0d;
    border: 1px solid #1e1e23;
    border-radius: 18px;
    overflow: hidden;
    box-shadow: 0 30px 80px rgba(2, 9, 45, 0.5);
    font-family: var(--font-sans);
  }
  .pv-top {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 15px 18px;
    border-bottom: 1px solid #16161a;
  }
  .pv-top .t {
    font-size: 12px;
    font-weight: 600;
    color: #d4d4d8;
    letter-spacing: -0.01em;
  }
  .pv-live {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: #22c55e;
  }
  .pv-live .d {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #22c55e;
    animation: pulse 2s var(--ease-in-out) infinite;
  }
  .pv-bal {
    padding: 20px 18px 16px;
    border-bottom: 1px solid #16161a;
  }
  .pv-bal .l {
    font-family: var(--font-mono);
    font-size: 9px;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: #63636e;
    margin-bottom: 9px;
  }
  .pv-bal .v {
    font-family: var(--font-mono);
    font-size: 30px;
    font-weight: 500;
    letter-spacing: -0.02em;
    color: #fafafa;
  }
  .pv-bal .v .u {
    font-size: 14px;
    color: #71717a;
  }
  .pv-bal .f {
    font-family: var(--font-mono);
    font-size: 12px;
    color: #52525b;
    margin-top: 5px;
  }
  .pv-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 13px 18px;
    border-bottom: 1px solid #16161a;
    border-left: 3px solid transparent;
  }
  .pv-row:last-child {
    border-bottom: 0;
  }
  .pv-row.ocean {
    background: linear-gradient(90deg, rgba(247, 147, 26, 0.1), transparent 82%);
    border-left-color: var(--btc-orange);
  }
  .pv-ic {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #0f0f11;
    border: 1px solid #1e1e23;
  }
  .pv-row.ocean .pv-ic {
    background: var(--btc-orange);
    border-color: var(--btc-orange);
  }
  .pv-ic img {
    width: 18px;
    height: 18px;
    display: block;
  }
  .pv-ic .material-symbols-sharp {
    font-size: 18px;
    color: #ffb300;
  }
  .pv-meta {
    flex: 1;
    min-width: 0;
  }
  .pv-t {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 13.5px;
    font-weight: 600;
    color: #fafafa;
  }
  .pv-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-family: var(--font-mono);
    font-size: 8px;
    font-weight: 500;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--btc-orange);
    background: rgba(247, 147, 26, 0.12);
    border: 1px solid rgba(247, 147, 26, 0.3);
    border-radius: var(--radius-pill);
    padding: 2px 7px 2px 5px;
  }
  .pv-chip img {
    height: 9px;
  }
  .pv-s {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 11.5px;
    color: #71717a;
    margin-top: 3px;
  }
  .pv-s .sdot {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: #3f3f46;
  }
  .pv-amt {
    text-align: right;
    flex-shrink: 0;
  }
  .pv-amt .v {
    display: block;
    font-family: var(--font-mono);
    font-size: 13.5px;
    font-weight: 500;
    color: #fafafa;
    white-space: nowrap;
  }
  .pv-amt .v.in {
    color: #22c55e;
  }
  .pv-amt .v.mat {
    color: #ffb300;
  }
  .pv-amt .v .u {
    font-size: 9px;
    color: #52525b;
  }
  .pv-amt .f {
    display: block;
    font-family: var(--font-mono);
    font-size: 10px;
    color: #52525b;
    margin-top: 3px;
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
