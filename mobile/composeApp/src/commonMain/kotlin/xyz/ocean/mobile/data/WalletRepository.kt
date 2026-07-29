package xyz.ocean.mobile.data

// The data seam the UI renders against. Two implementations:
//   • MockWalletRepository — the design's mock data (runs with no node).
//   • CoreWalletRepository — live data from the Rust core via UniFFI. Provided
//     per platform through `createCoreRepository` (Android wraps the JNA Kotlin
//     bindings today; iOS bridges via the Swift bindings — see README).
interface WalletRepository {
    val nowMs: Long
    val offer: String
    suspend fun pool(): PoolStats
    suspend fun workers(): List<Worker>
    suspend fun balances(): Balances
    suspend fun activity(): List<Tx>
}

class MockWalletRepository : WalletRepository {
    override val nowMs = Mock.NOW_MS
    override val offer = Mock.offer
    override suspend fun pool() = Mock.pool
    override suspend fun workers() = Mock.workers
    override suspend fun balances() = Mock.balances
    override suspend fun activity() = Mock.txs
}

// Build a core-backed repository rooted at the platform app-data dir, or null if
// the platform has no core binding wired yet (iOS falls back to Mock for now).
expect fun createCoreRepository(appDataDir: String): WalletRepository?
