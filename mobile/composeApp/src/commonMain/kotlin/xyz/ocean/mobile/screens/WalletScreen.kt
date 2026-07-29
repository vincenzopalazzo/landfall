package xyz.ocean.mobile.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
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
import xyz.ocean.mobile.data.Balances
import xyz.ocean.mobile.data.Tx
import xyz.ocean.mobile.data.TxStatus
import xyz.ocean.mobile.data.WalletRepository
import xyz.ocean.mobile.data.btc
import xyz.ocean.mobile.data.commas
import xyz.ocean.mobile.data.fmtUsd
import xyz.ocean.mobile.data.matEta
import xyz.ocean.mobile.data.usd
import xyz.ocean.mobile.theme.OceanColors
import xyz.ocean.mobile.theme.OceanType
import xyz.ocean.mobile.ui.BtnVariant
import xyz.ocean.mobile.ui.MaturityCard
import xyz.ocean.mobile.ui.OButton
import xyz.ocean.mobile.ui.OCard
import xyz.ocean.mobile.ui.OIcon
import xyz.ocean.mobile.ui.ProgressBar
import xyz.ocean.mobile.ui.SectionLabel
import xyz.ocean.mobile.ui.StatusDot
import xyz.ocean.mobile.ui.TxRow

@Composable
fun WalletScreen(
    repo: WalletRepository,
    usdUnit: Boolean,
    fullMode: Boolean,
    onSetMode: (Boolean) -> Unit,
    onOpenTx: (Tx) -> Unit,
    onSend: () -> Unit,
    onReceive: () -> Unit,
    onSeeAll: () -> Unit,
) {
    val bal by produceState<Balances?>(initialValue = null, repo) { value = repo.balances() }
    val txs by produceState<List<Tx>>(initialValue = emptyList(), repo) { value = repo.activity() }
    val b = bal ?: return
    val maturing = txs.firstOrNull { it.maturing }
    val recent = txs.filter { it.status != TxStatus.MATURING }.take(if (fullMode) 5 else 3)
    val shown = if (fullMode) b.total else b.channel

    Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp)) {
        Spacer(Modifier.height(4.dp))
        // simple/full segmented
        Segmented(fullMode, onSetMode)

        Spacer(Modifier.height(12.dp))
        // balance hero
        Column(
            Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp)).background(OceanColors.bgCard)
                .border(1.dp, OceanColors.border, RoundedCornerShape(16.dp)).padding(20.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(7.dp)) {
                StatusDot(OceanColors.success, 7)
                Text(
                    (if (fullMode) "TOTAL BALANCE · NODE ONLINE" else "SPENDABLE BALANCE"),
                    style = OceanType.monoXs.copy(letterSpacing = 0.8.sp),
                )
            }
            Spacer(Modifier.height(12.dp))
            Row(verticalAlignment = Alignment.Bottom) {
                Text(if (usdUnit) fmtUsd(usd(shown)) else commas(shown), style = OceanType.heroValue.copy(color = OceanColors.fgPrimary))
                if (!usdUnit) Text(" sats", style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 16.sp))
            }
            Spacer(Modifier.height(9.dp))
            val fiat = if (usdUnit) "${commas(shown)} sats" else fmtUsd(usd(shown))
            val matStr = if (maturing != null && !fullMode) " · +${commas(maturing.amt ?: 0)} maturing" else ""
            Text(fiat + matStr, style = OceanType.monoSm.copy(color = OceanColors.fgTertiary, fontSize = 13.sp))
            Spacer(Modifier.height(18.dp))
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Box(Modifier.weight(1f)) { OButton("Receive", BtnVariant.GHOST, icon = "in", fill = true, onClick = onReceive) }
                Box(Modifier.weight(1f)) { OButton("Send", BtnVariant.PRIMARY, icon = "out", fill = true, onClick = onSend) }
            }
        }

        // full: channel vs on-chain
        if (fullMode) {
            Spacer(Modifier.height(12.dp))
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                OCard(Modifier.weight(1f), padding = 15) {
                    SplitHead("bolt", "Lightning", OceanColors.accent, OceanColors.accentDim)
                    Spacer(Modifier.height(11.dp))
                    Row(verticalAlignment = Alignment.Bottom) {
                        Text(commas(b.channel), style = OceanType.splitValue.copy(color = OceanColors.fgPrimary))
                        Text(" sats", style = OceanType.monoXs)
                    }
                    Text("${fmtUsd(usd(b.channel))} spendable", style = OceanType.monoXs)
                    Spacer(Modifier.height(12.dp))
                    val capPct = (b.channel.toFloat() / b.capacity).coerceIn(0.04f, 1f)
                    ProgressBar(capPct)
                    Spacer(Modifier.height(7.dp))
                    Row(Modifier.fillMaxWidth()) {
                        Text(commas(b.channel), style = OceanType.monoXs.copy(color = OceanColors.fgTertiary))
                        Spacer(Modifier.weight(1f))
                        Text("${commas(b.capacity - b.channel)} inbound", style = OceanType.monoXs)
                    }
                }
                OCard(Modifier.weight(1f), padding = 15) {
                    SplitHead("btc", "On-chain", OceanColors.onchain, OceanColors.onchainDim)
                    Spacer(Modifier.height(11.dp))
                    Row(verticalAlignment = Alignment.Bottom) {
                        Text(btc(b.onchain), style = OceanType.splitValue.copy(color = OceanColors.fgPrimary))
                        Text(" BTC", style = OceanType.monoXs)
                    }
                    Text("${fmtUsd(usd(b.onchain))} confirmed", style = OceanType.monoXs)
                    Spacer(Modifier.height(12.dp))
                    Text("Held in your node wallet — withdraw to cold storage or open a channel.", style = OceanType.bodySm.copy(color = OceanColors.fgMuted, fontSize = 11.5.sp))
                }
            }
        }

        // maturity summary
        if (maturing != null) {
            Spacer(Modifier.height(12.dp))
            MaturityCard(maturing, showNote = fullMode)
        }

        // recent activity
        SectionLabel("Recent activity", more = "See all", onMore = onSeeAll)
        OCard(padding = 4) {
            recent.forEach { TxRow(it, usdUnit, repo.nowMs, compact = !fullMode, onOpen = onOpenTx) }
        }

        if (!fullMode) {
            Spacer(Modifier.height(14.dp))
            Row(
                Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).background(OceanColors.bgCard)
                    .border(1.dp, OceanColors.borderSubtle, RoundedCornerShape(12.dp))
                    .clickable { onSetMode(true) }.padding(14.dp),
                verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(13.dp),
            ) {
                Box(Modifier.size(30.dp).clip(RoundedCornerShape(8.dp)).background(OceanColors.bgSecondary).border(1.dp, OceanColors.border, RoundedCornerShape(8.dp)), contentAlignment = Alignment.Center) {
                    OIcon("chip", 16, OceanColors.fgSecondary)
                }
                Column(Modifier.weight(1f)) {
                    Text("Switch to full node view", style = OceanType.body.copy(fontWeight = FontWeight.Medium))
                    Text("Channels, on-chain, fees & more", style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 11.5.sp))
                }
                OIcon("chevR", 16, OceanColors.fgMuted)
            }
        }
        Spacer(Modifier.height(20.dp))
    }
}

@Composable
private fun Segmented(fullMode: Boolean, onSetMode: (Boolean) -> Unit) {
    Row(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(999.dp)).background(OceanColors.bgSecondary)
            .border(1.dp, OceanColors.border, RoundedCornerShape(999.dp)).padding(3.dp),
        horizontalArrangement = Arrangement.spacedBy(0.dp),
    ) {
        SegBtn("Simple", "wallet", !fullMode, Modifier.weight(1f)) { onSetMode(false) }
        SegBtn("Full node", "chip", fullMode, Modifier.weight(1f)) { onSetMode(true) }
    }
}

@Composable
private fun SegBtn(text: String, icon: String, on: Boolean, modifier: Modifier, onClick: () -> Unit) {
    Row(
        modifier.clip(RoundedCornerShape(999.dp)).background(if (on) OceanColors.accent else androidx.compose.ui.graphics.Color.Transparent)
            .clickable { onClick() }.padding(vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(6.dp, Alignment.CenterHorizontally),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        OIcon(icon, 15, if (on) OceanColors.onAccent else OceanColors.fgTertiary)
        Text(text, style = OceanType.body.copy(fontWeight = FontWeight.SemiBold, fontSize = 13.sp, color = if (on) OceanColors.onAccent else OceanColors.fgTertiary))
    }
}

@Composable
private fun SplitHead(icon: String, label: String, fg: androidx.compose.ui.graphics.Color, bg: androidx.compose.ui.graphics.Color) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Box(Modifier.size(26.dp).clip(RoundedCornerShape(7.dp)).background(bg), contentAlignment = Alignment.Center) {
            OIcon(icon, 15, fg)
        }
        Text(label, style = OceanType.bodySm.copy(color = OceanColors.fgSecondary, fontWeight = FontWeight.SemiBold))
    }
}
