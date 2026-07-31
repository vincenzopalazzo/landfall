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
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import xyz.ocean.mobile.data.fiatAvailable
import xyz.ocean.mobile.ui.OButton
import androidx.compose.runtime.CompositionLocalProvider
import xyz.ocean.mobile.data.LocalBtcUsd
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
 * Root of the OCEAN Lightning mobile app.
 *
 * [repo] is required and is always core-backed (`createCoreRepository`). There
 * is no fixture fallback: a build that cannot reach the wallet core shows an
 * error, it does not render a plausible fake wallet.
 */
@Composable
fun App(repo: WalletRepository) {
    var launch by remember { mutableStateOf<LaunchState>(LaunchState.Loading) }
    // Bumped by the retry button. `repo` alone can't key the effect below —
    // it never changes, so a retry would set Loading and then sit there.
    var reload by remember { mutableStateOf(0) }
    var tab by remember { mutableStateOf(Tab.POOL) }
    var usdUnit by remember { mutableStateOf(false) }
    var fullMode by remember { mutableStateOf(false) }
    var detail by remember { mutableStateOf<Tx?>(null) }
    var sheet by remember { mutableStateOf<String?>(null) } // "send" | "receive"
    // 0.0 = no rate available; the fiat toggle stays hidden and every amount
    // renders in sats. Never a hardcoded constant.
    var btcUsd by remember { mutableStateOf(0.0) }
    // Real node identity for the header. Null until read; the header then says
    // so instead of asserting "online".
    var node by remember { mutableStateOf<xyz.ocean.mobile.data.NodeInfo?>(null) }

    Box(Modifier.fillMaxSize().background(OceanColors.bgPrimary)) {
        LaunchedEffect(repo, reload) {
            // Fail closed: a core that errors here is NOT "no wallet yet".
            // Treating it as such would offer "Create a new wallet" to someone
            // who already has one.
            launch = runCatching {
                if (reload > 0) repo.refresh()
                val configured = repo.isWalletConfigured()
                val backupConfirmed = configured && repo.isBackupConfirmed()
                when {
                    !configured -> LaunchState.NeedsOnboarding(resumeBackup = false)
                    !backupConfirmed -> LaunchState.NeedsOnboarding(resumeBackup = true)
                    else -> LaunchState.Ready
                }
            }.getOrElse { LaunchState.Failed(it.message ?: "Could not read the wallet state.") }
        }
        LaunchedEffect(repo, reload) {
            btcUsd = runCatching { repo.btcUsd() }.getOrDefault(0.0)
        }
        LaunchedEffect(repo, launch) {
            if (launch is LaunchState.Ready) {
                node = runCatching { repo.nodeInfo() }.getOrNull()
            }
        }

        when (val state = launch) {
            is LaunchState.Loading -> return@Box
            is LaunchState.Failed -> {
                LaunchError(state.message, onRetry = { launch = LaunchState.Loading; reload += 1 })
                return@Box
            }
            is LaunchState.NeedsOnboarding -> {
                CompositionLocalProvider(LocalBtcUsd provides btcUsd) {
                    Onboarding(
                        repo = repo,
                        resumeBackup = state.resumeBackup,
                        onComplete = { launch = LaunchState.Ready },
                    )
                }
                return@Box
            }
            is LaunchState.Ready -> Unit
        }
        CompositionLocalProvider(LocalBtcUsd provides btcUsd) {
            Column(Modifier.fillMaxSize()) {
                AppHeader(
                    title = tab.title,
                    node = node,
                    usdUnit = usdUnit,
                    onToggleUnit = { usdUnit = !usdUnit },
                )
                Box(Modifier.weight(1f)) {
                    when (tab) {
                        Tab.POOL -> PoolScreen(repo, usdUnit, onOpenTx = { detail = it })
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
}

private sealed interface LaunchState {
    data object Loading : LaunchState
    data class NeedsOnboarding(val resumeBackup: Boolean) : LaunchState
    data object Ready : LaunchState
    data class Failed(val message: String) : LaunchState
}

/**
 * Shown when the wallet state can't be read. Deliberately offers only a retry:
 * the one thing it must never do is fall through to onboarding, which would
 * invite an existing wallet holder to generate a second seed.
 */
@Composable
private fun LaunchError(message: String, onRetry: () -> Unit) {
    Column(
        Modifier.fillMaxSize().padding(horizontal = 28.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        OIcon("warn", 34, OceanColors.warning)
        Text(
            "Could not open your wallet",
            style = OceanType.sheetTitle.copy(color = OceanColors.fgPrimary),
            modifier = Modifier.padding(top = 16.dp),
        )
        Text(
            message,
            style = OceanType.bodySm.copy(color = OceanColors.fgTertiary),
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = 10.dp, bottom = 20.dp),
        )
        OButton("Try again", icon = "refresh", onClick = onRetry)
    }
}

@Composable
private fun AppHeader(title: String, node: xyz.ocean.mobile.data.NodeInfo?, usdUnit: Boolean, onToggleUnit: () -> Unit) {
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
                // Reflects the actual node read, not a hardcoded "online".
                StatusDot(if (node != null) OceanColors.success else OceanColors.fgMuted, 6)
                Text(
                    node?.let { "${xyz.ocean.mobile.data.short(it.nodePk, 8, 4)} · online" }
                        ?: "connecting…",
                    style = OceanType.headSub,
                )
            }
        }
        Spacer(Modifier.weight(1f))
        // No live rate → no toggle. Offering "USD" with nothing behind it is
        // how a fabricated fiat figure reaches the screen.
        if (fiatAvailable()) {
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
