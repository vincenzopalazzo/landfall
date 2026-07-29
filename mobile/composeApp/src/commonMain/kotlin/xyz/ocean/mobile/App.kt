package xyz.ocean.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import xyz.ocean.mobile.data.MockWalletRepository
import xyz.ocean.mobile.data.Tx
import xyz.ocean.mobile.data.WalletRepository
import xyz.ocean.mobile.screens.ActivityScreen
import xyz.ocean.mobile.screens.NodeScreen
import xyz.ocean.mobile.screens.PoolScreen
import xyz.ocean.mobile.screens.WalletScreen
import xyz.ocean.mobile.sheets.ReceiveSheet
import xyz.ocean.mobile.sheets.SendSheet
import xyz.ocean.mobile.sheets.TxDetailSheet
import xyz.ocean.mobile.theme.OceanColors
import xyz.ocean.mobile.theme.OceanType
import xyz.ocean.mobile.ui.OIcon
import xyz.ocean.mobile.ui.StatusDot

enum class Tab(val key: String, val label: String, val icon: String, val title: String) {
    POOL("pool", "Pool", "pool", "Mining"),
    WALLET("wallet", "Wallet", "wallet", "Wallet"),
    ACTIVITY("activity", "Activity", "activity", "Activity"),
    NODE("node", "Node", "gear", "Node"),
}

/**
 * Root of the OCEAN Lightning mobile app. The [repo] is the data seam — pass a
 * [MockWalletRepository] to render the design's mock data, or a core-backed
 * repository (via `createCoreRepository`) for live node data.
 */
@Composable
fun App(repo: WalletRepository = MockWalletRepository()) {
    var onboarded by remember { mutableStateOf<Boolean?>(null) }
    var tab by remember { mutableStateOf(Tab.POOL) }
    var usdUnit by remember { mutableStateOf(false) }
    var fullMode by remember { mutableStateOf(false) }
    var detail by remember { mutableStateOf<Tx?>(null) }
    var sheet by remember { mutableStateOf<String?>(null) } // "send" | "receive"

    Box(Modifier.fillMaxSize().background(OceanColors.bgPrimary)) {
        LaunchedEffect(repo) {
            onboarded = runCatching { repo.isWalletConfigured() }.getOrDefault(false)
        }
        if (onboarded != true) {
            if (onboarded == false) {
                Onboarding(repo = repo, onComplete = { onboarded = true })
            }
            return@Box
        }
        Column(Modifier.fillMaxSize()) {
            AppHeader(
                title = tab.title,
                usdUnit = usdUnit,
                onToggleUnit = { usdUnit = !usdUnit },
            )
            Box(Modifier.weight(1f)) {
                when (tab) {
                    Tab.POOL -> PoolScreen(repo, usdUnit, onOpenTx = { detail = it }, onSeeWorkers = { tab = Tab.NODE })
                    Tab.WALLET -> WalletScreen(
                        repo, usdUnit, fullMode,
                        onSetMode = { fullMode = it },
                        onOpenTx = { detail = it },
                        onSend = { sheet = "send" },
                        onReceive = { sheet = "receive" },
                        onSeeAll = { tab = Tab.ACTIVITY },
                    )
                    Tab.ACTIVITY -> ActivityScreen(repo, usdUnit, onOpenTx = { detail = it })
                    Tab.NODE -> NodeScreen(repo, usdUnit, fullMode, onToggleUnit = { usdUnit = !usdUnit })
                }
            }
            TabBar(current = tab, onSelect = { tab = it })
        }

        detail?.let { TxDetailSheet(it, usdUnit, onClose = { detail = null }) }
        if (sheet == "send") SendSheet(repo, usdUnit, onClose = { sheet = null })
        if (sheet == "receive") ReceiveSheet(repo, onClose = { sheet = null })
    }
}

@Composable
private fun AppHeader(title: String, usdUnit: Boolean, onToggleUnit: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().padding(start = 20.dp, end = 20.dp, top = 58.dp, bottom = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Box(Modifier.size(28.dp).clip(RoundedCornerShape(8.dp)).background(OceanColors.accent), contentAlignment = Alignment.Center) {
            OIcon("spark", 17, OceanColors.onAccent)
        }
        Column {
            Text(title, style = OceanType.headTitle.copy(color = OceanColors.fgPrimary))
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                StatusDot(OceanColors.success, 6)
                Text("ocean-node · online", style = OceanType.headSub)
            }
        }
        Spacer(Modifier.weight(1f))
        Row(
            Modifier.clip(RoundedCornerShape(999.dp)).background(OceanColors.bgCard)
                .border(1.dp, OceanColors.border, RoundedCornerShape(999.dp))
                .clickable { onToggleUnit() }.padding(horizontal = 13.dp, vertical = 9.dp),
            verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            OIcon("swap", 14, OceanColors.fgSecondary)
            Text(if (usdUnit) "USD" else "sats", style = OceanType.monoSm.copy(fontSize = 12.sp))
        }
    }
}

@Composable
private fun TabBar(current: Tab, onSelect: (Tab) -> Unit) {
    Row(
        Modifier.fillMaxWidth().background(OceanColors.bgPrimary.copy(alpha = 0.92f))
            .border(0.dp, Color.Transparent).padding(start = 8.dp, end = 8.dp, top = 8.dp, bottom = 30.dp),
    ) {
        Tab.entries.forEach { t ->
            val on = t == current
            Column(
                Modifier.weight(1f).clickable { onSelect(t) }.padding(vertical = 6.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                OIcon(t.icon, 22, if (on) OceanColors.accent else OceanColors.fgMuted)
                Text(t.label, fontSize = 10.sp, fontWeight = FontWeight.SemiBold, color = if (on) OceanColors.accent else OceanColors.fgMuted)
            }
        }
    }
}
