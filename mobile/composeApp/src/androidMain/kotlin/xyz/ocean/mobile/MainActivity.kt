package xyz.ocean.mobile

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import xyz.ocean.mobile.data.MockWalletRepository
import xyz.ocean.mobile.data.createCoreRepository

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        // Seed + node data live under the app's private files dir. If the native
        // core lib isn't bundled yet (bindings not generated), fall back to the
        // design's mock data so the app still runs.
        val dir = filesDir.resolve("ocean").absolutePath
        val repo = runCatching { createCoreRepository(dir) }.getOrNull() ?: MockWalletRepository()
        setContent { App(repo) }
    }
}
