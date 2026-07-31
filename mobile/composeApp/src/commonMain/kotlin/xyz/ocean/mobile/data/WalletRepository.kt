package xyz.ocean.mobile.data

// The data seam the UI renders against. Backed by the Rust core over UniFFI
// (per platform via `createCoreRepository`), normally wrapped in
// `CachedWalletRepository` so switching tabs doesn't refetch.
//
// There is deliberately no mock/fixture implementation — an app that moves real
// funds must not be able to render plausible fake balances, and a build that
// cannot reach the core fails closed instead.
//
// Note the derived defaults: several screens want one field out of a larger
// core call, so the *call* is the interface method and the field accessors are
// defaults on top. That way one cache entry serves every caller instead of each
// accessor becoming its own round-trip.
interface WalletRepository {
    /** Epoch millis "now", read per call so relative times don't freeze. */
    val nowMs: Long

    // ── seed-derived state (one core call) ──
    suspend fun status(): WalletStatus
    suspend fun offer(): String? = status().offer
    suspend fun miningAddress(): String? = status().miningAddress
    suspend fun isWalletConfigured(): Boolean = status().configured

    // ── node state (one core call) ──
    suspend fun nodeInfo(): NodeInfo
    suspend fun balances(): Balances = nodeInfo().toBalances()

    suspend fun activity(): List<Tx>
    /** Live OCEAN mining stats, or null when no mining address is configured. */
    suspend fun pool(): PoolStats?
    /** Live BTC/USD spot; 0.0 when unavailable (callers then show sats). */
    suspend fun btcUsd(): Double

    // ── seed operations ──
    suspend fun generateWallet(): WalletSetup
    suspend fun restoreWallet(mnemonic: String): WalletSetup
    suspend fun revealSeed(): String
    suspend fun isBackupConfirmed(): Boolean
    suspend fun completeWalletSetup()
    suspend fun confirmBackup()

    // ── payments ──
    suspend fun payableAmountSats(payable: String): Long?
    suspend fun createInvoice(amountSats: Long? = null, description: String? = null): String
    suspend fun pay(payable: String, amountSats: Long? = null, note: String? = null): PaymentResult

    /**
     * Drop cached reads so the next call hits the core.
     *
     * Screens call this behind a Retry / pull-to-refresh so a user can always
     * force fresh data. A no-op on the uncached core repositories.
     */
    suspend fun refresh() = Unit
}

data class WalletSetup(
    val mnemonic: String?,
    val miningAddress: String,
)

data class PaymentResult(
    val id: String,
    val amountSats: Long,
)

/**
 * Build a core-backed repository rooted at the platform app-data dir.
 *
 * Throws if the native core is unavailable. It must never return a fixture
 * stand-in: callers surface the failure to the user instead.
 */
expect fun createCoreRepository(appDataDir: String): WalletRepository
