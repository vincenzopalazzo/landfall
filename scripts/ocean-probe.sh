#!/usr/bin/env bash
# Probe the public OCEAN API for a payout address and print exactly the values
# the dashboard + profile wire from it (see landfall-web/src/lib/StatsGrid.svelte).
# Lets you "run the API and see what it's doing" for any address.
#
# Usage: scripts/ocean-probe.sh <bc1q...address>
set -euo pipefail
ADDR="${1:?usage: scripts/ocean-probe.sh <bc1q...payout address>}"
BASE="https://api.ocean.xyz/v1"

echo "OCEAN API probe — $ADDR"
echo "  endpoints: statsnap, user_hashrate, earnpay, pool_stat"
echo
python3 - "$ADDR" "$BASE" <<'PY'
import sys, json, time, urllib.request
addr, base = sys.argv[1], sys.argv[2]

def get(path):
    with urllib.request.urlopen(f"{base}/{path}", timeout=15) as r:
        return json.load(r).get("result")

ss = get(f"statsnap/{addr}") or {}
uh = get(f"user_hashrate/{addr}") or {}
ep = get(f"earnpay/{addr}") or {}
ps = get("pool_stat") or {}

def f(x):
    try: return float(x)
    except (TypeError, ValueError): return 0.0
def hr(hps):
    hps = f(hps)
    for d, u in ((1e12, "Th/s"), (1e9, "Gh/s"), (1e6, "Mh/s")):
        if hps >= d: return f"{hps/d:.2f} {u}"
    return f"{int(hps)} h/s"

payouts = ep.get("payouts", []) or []
total = sum(round(f(p.get("total_satoshis_net_paid", 0))) for p in payouts)
ls = int(f(ss.get("lastest_share_ts") or uh.get("lastest_share_ts")))

print("Dashboard / profile would show:")
print(f"  Hashrate (5m):   {hr(ss.get('hashrate_300s'))}")
print(f"  Unpaid:          {round(f(ss.get('unpaid'))*1e8):,} sats")
print(f"  Total paid:      {total:,} sats  ({len(payouts)} payouts)")
print(f"  Est. next block: {round(f(ss.get('estimated_payout_next_block'))*1e8):,} sats")
print(f"  Workers:         {uh.get('active_worker_count', 0)}")
print(f"  1h avg:          {hr(uh.get('hashrate_3600s'))}")
print(f"  24h avg:         {hr(uh.get('hashrate_86400s'))}")
print(f"  TIDES shares:    {int(f(ss.get('shares_in_tides'))):,}")
print(f"  Last share:      {int(time.time()-ls)}s ago" if ls else "  Last share:      —")
print(f"  Status:          {'Mining' if (uh.get('active_worker_count',0) or f(ss.get('hashrate_300s'))>0) else 'Idle'}")
print(f"  Pool:            {int(f(ps.get('active_users'))):,} miners · "
      f"diff {f(ps.get('network_difficulty'))/1e12:.1f}T · "
      f"reward {f(ps.get('current_estimated_block_reward')):.3f} BTC")
PY
