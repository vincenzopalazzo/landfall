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
import xyz.ocean.mobile.data.NodeInfo
import xyz.ocean.mobile.data.WalletRepository
import xyz.ocean.mobile.data.commas
import xyz.ocean.mobile.data.short
import xyz.ocean.mobile.theme.OceanColors
import xyz.ocean.mobile.theme.OceanType
import xyz.ocean.mobile.ui.OIcon
import xyz.ocean.mobile.ui.Pill
import xyz.ocean.mobile.ui.PillTone
import xyz.ocean.mobile.ui.SectionLabel
import xyz.ocean.mobile.ui.StatusDot

@Composable
fun NodeScreen(repo: WalletRepository, usdUnit: Boolean, fullMode: Boolean, onToggleUnit: () -> Unit) {
    // Node identity, channel counts and the payout offer all come off the core.
    // Nothing on this screen is a written-in constant any more.
    val node by produceState<NodeInfo?>(initialValue = null, repo) {
        value = runCatching { repo.nodeInfo() }.getOrNull()
    }
    val offer by produceState<String?>(initialValue = null, repo) {
        value = runCatching { repo.offer() }.getOrNull()
    }
    val backedUp by produceState<Boolean?>(initialValue = null, repo) {
        value = runCatching { repo.isBackupConfirmed() }.getOrNull()
    }
    Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp)) {
        Spacer(Modifier.height(6.dp))
        // node status card
        Column(
            Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).background(OceanColors.bgCard)
                .border(1.dp, OceanColors.borderSubtle, RoundedCornerShape(12.dp)).padding(16.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Box(Modifier.size(40.dp).clip(RoundedCornerShape(10.dp)).background(OceanColors.accentDim), contentAlignment = Alignment.Center) {
                    OIcon("node", 20, OceanColors.accent)
                }
                Column(Modifier.weight(1f)) {
                    Text(
                        if (node != null) short(node!!.nodePk, 10, 6) else "Lexe node",
                        style = OceanType.body.copy(fontWeight = FontWeight.SemiBold, fontSize = 15.sp),
                    )
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        StatusDot(if (node != null) OceanColors.success else OceanColors.fgMuted, 6)
                        Text(
                            node?.let { "Online · ${it.usableChannels}/${it.channels} channels usable" }
                                ?: "Reading node status…",
                            style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 11.5.sp),
                        )
                    }
                }
                if (node != null) Pill(PillTone.OK, "Online") else Pill(PillTone.MUTED, "…")
            }
        }

        SectionLabel("Payouts")
        SettingsGroup {
            // "This device's offer", not "the OCEAN payout offer": on a restored
            // wallet this is a freshly minted offer, not necessarily the one
            // OCEAN is configured to pay.
            SettingRow(
                "offer", "This device's payout offer",
                offer?.let { short(it, 14, 10) } ?: "not created yet",
            )
            SettingRow("shield", "Payout verification", "Offer + payer-note format check")
        }

        SectionLabel("Wallet")
        SettingsGroup {
            SettingRow("swap", "Display unit", if (usdUnit) "US Dollar" else "Bitcoin (sats)", toggle = usdUnit, onToggle = onToggleUnit)
            SettingRow("wallet", "Default view", if (fullMode) "Full node" else "Simple wallet")
            SettingRow(
                "bolt", "Channels & liquidity",
                node?.let { "${it.channels} channels · ${commas(it.lightningTotalSats)} sats capacity" }
                    ?: "unavailable",
            )
        }

        SectionLabel("Security")
        SettingsGroup {
            SettingRow(
                "key", "Backup & recovery",
                when (backedUp) {
                    true -> "Recovery phrase confirmed"
                    false -> "Recovery phrase not confirmed"
                    null -> "checking…"
                },
            )
            SettingRow("ext", "Open pool dashboard", "ocean.xyz")
        }

        Spacer(Modifier.height(24.dp))
        Text("OCEAN Lightning · v0.9.0 (early access)", style = OceanType.monoXs.copy(color = OceanColors.fgMuted), modifier = Modifier.fillMaxWidth().padding(bottom = 8.dp))
        Spacer(Modifier.height(20.dp))
    }
}

@Composable
private fun SettingsGroup(content: @Composable () -> Unit) {
    Column(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).background(OceanColors.bgCard)
            .border(1.dp, OceanColors.borderSubtle, RoundedCornerShape(12.dp)),
    ) { content() }
}

@Composable
private fun SettingRow(icon: String, title: String, sub: String?, toggle: Boolean? = null, onToggle: (() -> Unit)? = null) {
    Row(
        Modifier.fillMaxWidth().then(if (onToggle != null) Modifier.clickable { onToggle() } else Modifier).padding(16.dp),
        verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(13.dp),
    ) {
        Box(Modifier.size(30.dp).clip(RoundedCornerShape(8.dp)).background(OceanColors.bgSecondary).border(1.dp, OceanColors.border, RoundedCornerShape(8.dp)), contentAlignment = Alignment.Center) {
            OIcon(icon, 16, OceanColors.fgSecondary)
        }
        Column(Modifier.weight(1f)) {
            Text(title, style = OceanType.body.copy(fontSize = 14.5.sp))
            if (sub != null) Text(sub, style = OceanType.monoXs.copy(color = OceanColors.fgTertiary))
        }
        if (toggle != null) Toggle(toggle) else OIcon("chevR", 16, OceanColors.fgMuted)
    }
}

@Composable
private fun Toggle(on: Boolean) {
    Box(
        Modifier.size(width = 42.dp, height = 25.dp).clip(RoundedCornerShape(999.dp))
            .background(if (on) OceanColors.accent else OceanColors.borderStrong),
        contentAlignment = if (on) Alignment.CenterEnd else Alignment.CenterStart,
    ) {
        Box(Modifier.padding(2.dp).size(21.dp).clip(RoundedCornerShape(999.dp)).background(if (on) OceanColors.onAccent else androidx.compose.ui.graphics.Color.White))
    }
}
