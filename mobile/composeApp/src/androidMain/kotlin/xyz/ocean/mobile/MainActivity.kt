package xyz.ocean.mobile

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import xyz.ocean.mobile.data.MockWalletRepository
import xyz.ocean.mobile.data.createCoreRepository
import xyz.ocean.mobile.theme.OceanColors
import xyz.ocean.mobile.theme.OceanType

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        // Seed + node data live under the app's private files dir. Production
        // must fail closed if the native core cannot load; fixtures are allowed
        // only in an explicit debug build.
        val dir = filesDir.resolve("ocean").absolutePath
        val repo = runCatching { createCoreRepository(dir) }
        setContent {
            repo.fold(
                onSuccess = { App(it) },
                onFailure = { error ->
                    if (BuildConfig.DEBUG) App(MockWalletRepository())
                    else Box(
                        Modifier.fillMaxSize().background(OceanColors.bgPrimary).padding(28.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            "OCEAN Lightning could not load its secure wallet core. Reinstall the app or contact support.\n\n${error.message.orEmpty()}",
                            style = OceanType.body.copy(color = OceanColors.error),
                        )
                    }
                },
            )
        }
    }
}
