package xyz.ocean.mobile.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import xyz.ocean.mobile.data.Dir
import xyz.ocean.mobile.data.Tx
import xyz.ocean.mobile.data.WalletRepository
import xyz.ocean.mobile.theme.OceanColors
import xyz.ocean.mobile.theme.OceanType
import xyz.ocean.mobile.ui.OCard
import xyz.ocean.mobile.ui.OIcon
import xyz.ocean.mobile.ui.TxRow

@Composable
fun ActivityScreen(repo: WalletRepository, usdUnit: Boolean, onOpenTx: (Tx) -> Unit) {
    val txs by produceState<List<Tx>>(initialValue = emptyList(), repo) { value = repo.activity() }
    var filter by remember { mutableStateOf("all") }
    val oceanCount = txs.count { it.isOcean }

    val rows = txs.filter {
        when (filter) {
            "ocean" -> it.isOcean
            "in" -> it.dir == Dir.IN
            "out" -> it.dir == Dir.OUT
            else -> true
        }
    }
    val ocean = rows.filter { it.isOcean }
    val rest = rows.filter { !it.isOcean }

    Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp)) {
        Spacer(Modifier.height(6.dp))
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Chip("All", filter == "all", false, null) { filter = "all" }
            Chip("OCEAN payouts", filter == "ocean", true, oceanCount) { filter = "ocean" }
            Chip("Received", filter == "in", false, null) { filter = "in" }
            Chip("Sent", filter == "out", false, null) { filter = "out" }
        }
        Spacer(Modifier.height(14.dp))
        if (rows.isEmpty()) {
            Column(Modifier.fillMaxWidth().padding(vertical = 44.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                OIcon("search", 28, OceanColors.borderStrong)
                Spacer(Modifier.height(10.dp))
                Text("No matching activity", style = OceanType.bodySm.copy(color = OceanColors.fgMuted))
            }
        } else {
            OCard(padding = 4) {
                if (filter == "all" && ocean.isNotEmpty()) {
                    GroupLabel("OCEAN payouts", OceanColors.accent)
                    ocean.forEach { TxRow(it, usdUnit, repo.nowMs, onOpen = onOpenTx) }
                    GroupLabel("Other activity", OceanColors.fgMuted)
                    rest.forEach { TxRow(it, usdUnit, repo.nowMs, onOpen = onOpenTx) }
                } else {
                    rows.forEach { TxRow(it, usdUnit, repo.nowMs, onOpen = onOpenTx) }
                }
            }
        }
        Spacer(Modifier.height(20.dp))
    }
}

@Composable
private fun GroupLabel(text: String, color: Color) {
    Text(text.uppercase(), style = OceanType.sectionLabel.copy(color = color), modifier = Modifier.padding(start = 4.dp, top = 10.dp, bottom = 4.dp))
}

@Composable
private fun Chip(text: String, on: Boolean, ocean: Boolean, count: Int?, onClick: () -> Unit) {
    val fg = when {
        ocean && on -> OceanColors.accent
        on -> OceanColors.fgPrimary
        else -> OceanColors.fgTertiary
    }
    val bg = when {
        ocean && on -> OceanColors.accentDim
        on -> OceanColors.bgCardHover
        else -> Color.Transparent
    }
    Row(
        Modifier.clip(RoundedCornerShape(999.dp)).background(bg)
            .border(1.dp, if (on) OceanColors.fgMuted else OceanColors.borderStrong, RoundedCornerShape(999.dp))
            .clickable { onClick() }.padding(horizontal = 13.dp, vertical = 7.dp),
        verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        if (ocean) OIcon("spark", 12, fg)
        Text(text, style = OceanType.bodySm.copy(color = fg, fontSize = 12.5.sp, fontWeight = FontWeight.Medium))
        if (count != null) Text("$count", style = OceanType.monoXs.copy(color = fg))
    }
}
