package xyz.ocean.mobile.data

// The data seam the UI renders against. Two implementations:
//   • MockWalletRepository — the design's mock data (runs with no node).
//   • CoreWalletRepository — live data from the Rust core via UniFFI. Provided
//     per platform through `createCoreRepository` (Android wraps the JNA Kotlin
//     bindings today; iOS bridges via the Swift bindings — see README).
interface WalletRepository {
    val nowMs: Long
    val offer: String?
    val miningAddress: String?
    suspend fun isWalletConfigured(): Boolean
    suspend fun generateWallet(): WalletSetup
    suspend fun restoreWallet(mnemonic: String): WalletSetup
    suspend fun revealSeed(): String
    suspend fun isBackupConfirmed(): Boolean
    suspend fun completeWalletSetup()
    suspend fun confirmBackup()
    suspend fun payableAmountSats(payable: String): Long?
    suspend fun createInvoice(amountSats: Long? = null, description: String? = null): String
    suspend fun pay(payable: String, amountSats: Long? = null, note: String? = null): PaymentResult
    suspend fun pool(): PoolStats
    suspend fun workers(): List<Worker>
    suspend fun balances(): Balances
    suspend fun activity(): List<Tx>
}

class MockWalletRepository : WalletRepository {
    private var configured = false
    override val nowMs = Mock.NOW_MS
    override val offer = Mock.offer
    override val miningAddress = Mock.miningAddress
    override suspend fun isWalletConfigured() = configured
    override suspend fun generateWallet(): WalletSetup {
        configured = true
        return WalletSetup(Mock.mnemonic, Mock.miningAddress)
    }
    override suspend fun restoreWallet(mnemonic: String): WalletSetup {
        configured = true
        return WalletSetup(null, Mock.miningAddress)
    }
    override suspend fun revealSeed() = Mock.mnemonic
    override suspend fun isBackupConfirmed() = false
    override suspend fun completeWalletSetup() = Unit
    override suspend fun confirmBackup() = Unit
    override suspend fun payableAmountSats(payable: String): Long? = null
    override suspend fun createInvoice(amountSats: Long?, description: String?) = Mock.invoice
    override suspend fun pay(payable: String, amountSats: Long?, note: String?) =
        PaymentResult("mock-payment-reference", amountSats ?: 0L)
    override suspend fun pool() = Mock.pool
    override suspend fun workers() = Mock.workers
    override suspend fun balances() = Mock.balances
    override suspend fun activity() = Mock.txs
}

data class WalletSetup(
    val mnemonic: String?,
    val miningAddress: String,
)

data class PaymentResult(
    val id: String,
    val amountSats: Long,
)

// Build a core-backed repository rooted at the platform app-data dir, or null if
// the platform has no core binding wired yet (iOS falls back to Mock for now).
expect fun createCoreRepository(appDataDir: String): WalletRepository?
