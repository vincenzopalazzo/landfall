package xyz.ocean.mobile.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import xyz.ocean.mobile.data.Balances
import xyz.ocean.mobile.data.PoolStats
import xyz.ocean.mobile.data.Tx
import xyz.ocean.mobile.data.WalletRepository
import xyz.ocean.mobile.data.bestHashrate
import xyz.ocean.mobile.data.commas
import xyz.ocean.mobile.data.fiatOrNull
import xyz.ocean.mobile.data.hashrate
import xyz.ocean.mobile.data.pct
import xyz.ocean.mobile.data.rel
import xyz.ocean.mobile.theme.OceanColors
import xyz.ocean.mobile.theme.OceanType
import xyz.ocean.mobile.ui.OCard
import xyz.ocean.mobile.ui.OButton
import xyz.ocean.mobile.ui.OIcon
import xyz.ocean.mobile.ui.SectionLabel
import xyz.ocean.mobile.ui.StatCard
import xyz.ocean.mobile.ui.StatusDot

/**
 * Mining overview. Every figure is live: hashrate/workers/shares/balances come
 * from OCEAN's public API (`OceanlnCore.poolStats`), spendable balance from the
 * node.
 *
 * The design's pool-wide hashrate, blocks-found, last-block and reject-rate
 * tiles are gone: OCEAN's public API exposes none of them, and this screen sits
 * next to a real balance, so a decorative number here reads as a fact.
 */
@Composable
fun PoolScreen(repo: WalletRepository, usdUnit: Boolean, onOpenTx: (Tx) -> Unit) {
    var retry by remember { mutableStateOf(0) }
    val loaded by produceState<Result<PoolData>?>(initialValue = null, repo, retry) {
        value = runCatching {
            // An explicit Retry means the user wants fresh data, so drop the
            // cache first rather than handing back what just failed to satisfy.
            if (retry > 0) repo.refresh()
            PoolData(repo.pool(), repo.balances(), repo.nowMs)
        }
    }
    val result = loaded
    if (result == null) {
        Box(Modifier.fillMaxWidth().padding(48.dp), contentAlignment = Alignment.Center) {
            androidx.compose.material3.CircularProgressIndicator(color = OceanColors.accent)
        }
        return
    }
    if (result.isFailure) {
        LoadFailure(result.exceptionOrNull()?.message) { retry += 1 }
        return
    }
    val data = result.getOrThrow()
    val p = data.pool
    val b = data.balances

    Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp)) {
        Spacer(Modifier.height(6.dp))

        if (p == null) {
            NoMiningAddress()
            Spacer(Modifier.height(12.dp))
        } else {
            HashrateHero(p)
            Spacer(Modifier.height(12.dp))
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                StatCard(
                    "spark", "Unpaid", commas(p.unpaidSats), "sats",
                    fiatOrNull(p.unpaidSats) ?: "awaiting payout",
                    OceanColors.accent, Modifier.weight(1f),
                )
                StatCard(
                    "hourglass", "Est. next block", commas(p.estPayoutNextBlockSats), "sats",
                    fiatOrNull(p.estPayoutNextBlockSats) ?: "if OCEAN finds one",
                    OceanColors.warning, Modifier.weight(1f),
                )
            }
            Spacer(Modifier.height(10.dp))
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                StatCard(
                    "wallet", "Spendable", commas(b.channel), "sats",
                    fiatOrNull(b.channel) ?: "on your node", modifier = Modifier.weight(1f),
                )
                StatCard(
                    "btc", "Lifetime", commas(p.lifetimeSats), "sats",
                    fiatOrNull(p.lifetimeSats) ?: "paid + unpaid", modifier = Modifier.weight(1f),
                )
            }

            SectionLabel("Pool")
            OCard {
                Row(Modifier.fillMaxWidth()) {
                    PoolStatCell("Your share", pct(p.sharePct), "%", Modifier.weight(1f))
                    PoolStatCell("Active miners", commas(p.poolActiveUsers), "", Modifier.weight(1f))
                    PoolStatCell("Pool workers", commas(p.poolActiveWorkers), "", Modifier.weight(1f))
                }
                if (p.lastShareTs > 0) {
                    Spacer(Modifier.height(13.dp))
                    Row(
                        Modifier.fillMaxWidth().padding(top = 13.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        OIcon("cube", 14, OceanColors.accent)
                        Text(
                            "Last accepted share ${rel(p.lastShareTs * 1000L, data.nowMs)}",
                            style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 12.sp),
                        )
                    }
                }
            }
        }
        Spacer(Modifier.height(20.dp))
    }
}

private data class PoolData(
    val pool: PoolStats?,
    val balances: Balances,
    val nowMs: Long,
)

@Composable
private fun HashrateHero(p: PoolStats) {
    // Shortest window with an actual reading. Pinning this to 5m reported
    // "0 H/s" for a rig that had merely paused.
    val best = p.bestHashrate()
    val (value, unit) = hashrate(best?.first ?: 0.0)
    val window = best?.second ?: "5 MIN"
    Column(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp)).background(OceanColors.bgCard)
            .border(1.dp, OceanColors.border, RoundedCornerShape(16.dp)).padding(20.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(7.dp)) {
            OIcon("chip", 12, OceanColors.fgMuted)
            Text("HASHRATE · $window AVG", style = OceanType.monoXs.copy(letterSpacing = 0.8.sp))
        }
        Spacer(Modifier.height(12.dp))
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.Bottom) {
            Row(verticalAlignment = Alignment.Bottom) {
                Text(value, style = OceanType.hashrateValue.copy(color = OceanColors.fgPrimary))
                Text(" $unit", style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 15.sp))
            }
            Spacer(Modifier.weight(1f))
            // Real windows, not a decorative sparkline: 24h / 3h / 1h / 5m.
            WindowBars(p)
        }
        Spacer(Modifier.height(12.dp))
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            // A rig between shares reports 0 active workers while still having
            // a real 3h/24h average, so "no workers online" alone read as "not
            // mining" when the miner was simply idle for a moment.
            val mining = best != null
            StatusDot(
                when {
                    p.activeWorkers > 0 -> OceanColors.success
                    mining -> OceanColors.warning
                    else -> OceanColors.fgMuted
                },
            )
            Text(
                when {
                    p.activeWorkers > 0 ->
                        "${p.activeWorkers} worker${if (p.activeWorkers == 1) "" else "s"} online"
                    mining -> "no workers reporting right now"
                    else -> "not mining"
                },
                style = OceanType.bodySm.copy(color = OceanColors.fgTertiary),
            )
        }
    }
}

/**
 * The four hashrate windows OCEAN actually reports, drawn to scale against the
 * largest of them. Replaces the design's fixed decorative sparkline — there is
 * no per-minute history endpoint to draw a real one from.
 */
@Composable
private fun WindowBars(p: PoolStats) {
    val windows = listOf(
        "24h" to p.hashrate86400s,
        "3h" to p.hashrate10800s,
        "1h" to p.hashrate3600s,
        "5m" to p.hashrate300s,
    )
    val peak = windows.maxOf { it.second }
    if (peak <= 0.0) return
    Row(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.Bottom) {
        windows.forEach { (label, v) ->
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Box(
                    Modifier.width(10.dp)
                        .height((6 + (38 * (v / peak)).toInt()).dp)
                        .clip(RoundedCornerShape(2.dp))
                        .background(OceanColors.accent.copy(alpha = 0.45f + 0.45f * (v / peak).toFloat())),
                )
                Text(label, style = OceanType.monoXs.copy(fontSize = 8.sp), modifier = Modifier.padding(top = 3.dp))
            }
        }
    }
}

@Composable
private fun NoMiningAddress() {
    OCard {
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            OIcon("info", 16, OceanColors.accent)
            Column {
                Text("No mining address yet", style = OceanType.body.copy(color = OceanColors.fgPrimary))
                Text(
                    "Mining stats appear once your wallet has derived a payout address and OCEAN has seen shares from it.",
                    style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 12.sp),
                    modifier = Modifier.padding(top = 4.dp),
                )
            }
        }
    }
}

@Composable
internal fun LoadFailure(message: String?, onRetry: () -> Unit) {
    Column(
        Modifier.fillMaxWidth().padding(horizontal = 24.dp, vertical = 48.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        OIcon("warn", 30, OceanColors.warning)
        Text(
            "Could not load live data",
            style = OceanType.sheetTitle.copy(color = OceanColors.fgPrimary),
            modifier = Modifier.padding(top = 14.dp),
        )
        Text(
            message ?: "Check your connection and try again.",
            style = OceanType.bodySm.copy(color = OceanColors.fgTertiary),
            modifier = Modifier.padding(top = 8.dp, bottom = 18.dp),
        )
        OButton("Retry", icon = "refresh", onClick = onRetry)
    }
}

@Composable
private fun PoolStatCell(label: String, value: String, unit: String, modifier: Modifier = Modifier) {
    Column(modifier) {
        Text(label, style = OceanType.monoXs.copy(color = OceanColors.fgMuted))
        Spacer(Modifier.height(6.dp))
        Row(verticalAlignment = Alignment.Bottom) {
            Text(value, style = OceanType.splitValue.copy(color = OceanColors.fgPrimary))
            if (unit.isNotEmpty()) Text(unit, style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 11.sp))
        }
    }
}
