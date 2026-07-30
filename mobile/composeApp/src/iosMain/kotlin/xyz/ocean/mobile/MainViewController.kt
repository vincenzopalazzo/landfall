package xyz.ocean.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.ComposeUIViewController
import xyz.ocean.mobile.data.WalletCoreBridge
import xyz.ocean.mobile.data.cached
import xyz.ocean.mobile.data.createCoreRepository
import xyz.ocean.mobile.data.installedBridge
import xyz.ocean.mobile.theme.OceanColors
import xyz.ocean.mobile.theme.OceanType

/**
 * Entry point the iOS host embeds (`iosApp/ContentView.swift`).
 *
 * [bridge] is the Swift implementation of [WalletCoreBridge] wrapping the
 * UniFFI-generated `OceanlnCore` — see `iosApp/iosApp/CoreBridge.swift`. It is
 * required: the same posture as Android's `MainActivity`, where the wallet core
 * is the only data source and a failure to reach it is shown, not papered over
 * with fixtures.
 */
fun MainViewController(bridge: WalletCoreBridge) = ComposeUIViewController {
    installedBridge = bridge
    // `.cached()` keeps tab switches from refetching; see CachedWalletRepository.
    val repo = runCatching { createCoreRepository("").cached() }
    repo.fold(
        onSuccess = { App(it) },
        onFailure = { error ->
            Box(
                Modifier.fillMaxSize().background(OceanColors.bgPrimary).padding(28.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    "OCEAN Lightning could not load its secure wallet core.\n\n" +
                        error.message.orEmpty() +
                        "\n\nYour wallet has not been changed. Restart the app and try again.",
                    style = OceanType.body.copy(color = OceanColors.error),
                )
            }
        },
    )
}
