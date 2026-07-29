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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import xyz.ocean.mobile.data.Tx
import xyz.ocean.mobile.data.TxStatus
import xyz.ocean.mobile.data.WalletRepository
import xyz.ocean.mobile.data.btc
import xyz.ocean.mobile.data.commas
import xyz.ocean.mobile.data.fmtUsd
import xyz.ocean.mobile.data.usd
import xyz.ocean.mobile.theme.OceanColors
import xyz.ocean.mobile.theme.OceanType
import xyz.ocean.mobile.ui.MaturityCard
import xyz.ocean.mobile.ui.OCard
import xyz.ocean.mobile.ui.OIcon
import xyz.ocean.mobile.ui.SectionLabel
import xyz.ocean.mobile.ui.StatCard
import xyz.ocean.mobile.ui.StatusDot

@Composable
fun PoolScreen(repo: WalletRepository, usdUnit: Boolean, onOpenTx: (Tx) -> Unit, onSeeWorkers: () -> Unit) {
    val pool by produceState<xyz.ocean.mobile.data.PoolStats?>(initialValue = null, repo) { value = repo.pool() }
    val workers by produceState<List<xyz.ocean.mobile.data.Worker>>(initialValue = emptyList(), repo) { value = repo.workers() }
    val bal by produceState<xyz.ocean.mobile.data.Balances?>(initialValue = null, repo) { value = repo.balances() }
    val txs by produceState<List<Tx>>(initialValue = emptyList(), repo) { value = repo.activity() }
    val p = pool ?: return
    val b = bal ?: return
    val maturing = txs.firstOrNull { it.maturing }

    Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp)) {
        Spacer(Modifier.height(6.dp))
        // hashrate hero
        Column(
            Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp)).background(OceanColors.bgCard)
                .border(1.dp, OceanColors.border, RoundedCornerShape(16.dp)).padding(20.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(7.dp)) {
                OIcon("chip", 12, OceanColors.fgMuted)
                Text("TOTAL HASHRATE · LIVE", style = OceanType.monoXs.copy(letterSpacing = 0.8.sp))
            }
            Spacer(Modifier.height(12.dp))
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.Bottom) {
                Row(verticalAlignment = Alignment.Bottom) {
                    Text(p.hashrate.toString(), style = OceanType.hashrateValue.copy(color = OceanColors.fgPrimary))
                    Text(" ${p.unit}", style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 15.sp))
                }
                Spacer(Modifier.weight(1f))
                Sparkline()
            }
            Spacer(Modifier.height(12.dp))
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                StatusDot(OceanColors.success)
                Text("${p.workersOnline}/${p.workersTotal} workers online · ${p.rejectPct}% reject", style = OceanType.bodySm.copy(color = OceanColors.fgTertiary))
            }
        }

        Spacer(Modifier.height(12.dp))
        // stats grid 2x2
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            StatCard("spark", "Earned · 24h", "+${commas(p.earned24h)}", "sats", fmtUsd(usd(p.earned24h)), OceanColors.success, Modifier.weight(1f))
            StatCard("hourglass", "Maturing", if (maturing != null) commas(maturing.amt ?: 0) else "0", "sats",
                if (maturing != null) "matures ${xyz.ocean.mobile.data.matEta(maturing.mat!!)}" else "nothing pending",
                OceanColors.warning, Modifier.weight(1f))
        }
        Spacer(Modifier.height(10.dp))
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            StatCard("wallet", "Spendable", commas(b.channel), "sats", fmtUsd(usd(b.channel)), modifier = Modifier.weight(1f))
            StatCard("btc", "Lifetime paid", btc(p.lifetimePaid), "BTC", fmtUsd(usd(p.lifetimePaid)), modifier = Modifier.weight(1f))
        }

        // maturity explainer
        if (maturing != null) {
            SectionLabel("Next payout")
            MaturityCard(maturing, showNote = true)
        }

        // pool position
        SectionLabel("Pool")
        OCard {
            Row(Modifier.fillMaxWidth()) {
                PoolStat("Your share", "${p.sharePct}", "%", Modifier.weight(1f))
                PoolStat("Pool hashrate", p.poolHashrate, " ${p.poolUnit}", Modifier.weight(1f))
                PoolStat("Blocks found", commas(p.blocksFound), "", Modifier.weight(1f))
            }
            Spacer(Modifier.height(13.dp))
            Row(Modifier.fillMaxWidth().padding(top = 13.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OIcon("cube", 14, OceanColors.accent)
                Text("Last block ${commas(p.lastBlock)} · ${p.lastBlockAgo}", style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 12.sp))
            }
        }

        // workers
        SectionLabel("Workers", more = "All workers", onMore = onSeeWorkers)
        OCard {
            workers.forEachIndexed { i, w ->
                Row(Modifier.fillMaxWidth().padding(vertical = 13.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    Box(Modifier.size(34.dp).clip(RoundedCornerShape(9.dp)).background(OceanColors.bgSecondary).border(1.dp, OceanColors.border, RoundedCornerShape(9.dp)), contentAlignment = Alignment.Center) {
                        OIcon("chip", 18, if (w.online) OceanColors.fgTertiary else OceanColors.fgMuted)
                    }
                    Column(Modifier.weight(1f)) {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Text(w.id, style = OceanType.body.copy(fontWeight = FontWeight.SemiBold))
                            StatusDot(if (w.online) OceanColors.success else OceanColors.fgMuted, 6)
                        }
                        val meta = buildString {
                            append(w.model)
                            if (w.temp != null) append(" · ${w.temp}°C")
                            append(" · ${w.ago}")
                        }
                        Text(meta, style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 11.5.sp))
                    }
                    if (w.online) Text("${w.hr} Th/s", style = OceanType.monoSm.copy(color = OceanColors.fgPrimary, fontSize = 14.sp))
                    else Text("offline", style = OceanType.monoSm.copy(color = OceanColors.fgMuted, fontSize = 14.sp))
                }
                if (i != workers.lastIndex) Box(Modifier.fillMaxWidth().height(1.dp).background(OceanColors.borderSubtle))
            }
        }
        Spacer(Modifier.height(20.dp))
    }
}

@Composable
private fun PoolStat(label: String, value: String, unit: String, modifier: Modifier = Modifier) {
    Column(modifier) {
        Text(label, style = OceanType.monoXs.copy(color = OceanColors.fgMuted))
        Spacer(Modifier.height(6.dp))
        Row(verticalAlignment = Alignment.Bottom) {
            Text(value, style = OceanType.splitValue.copy(color = OceanColors.fgPrimary))
            if (unit.isNotEmpty()) Text(unit, style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 11.sp))
        }
    }
}

@Composable
private fun Sparkline() {
    val bars = listOf(58, 62, 60, 66, 63, 70, 68, 65, 72, 69, 74, 71)
    Row(Modifier.height(44.dp), verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(3.dp)) {
        bars.forEachIndexed { i, h ->
            Box(
                Modifier.width(6.dp).height((44 * h / 100).dp).clip(RoundedCornerShape(1.5.dp))
                    .background(OceanColors.accent.copy(alpha = 0.35f + (i.toFloat() / bars.size) * 0.5f)),
            )
        }
    }
}
