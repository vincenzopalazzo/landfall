package xyz.ocean.mobile.data

/**
 * The iOS seam to the Rust core.
 *
 * Kotlin/Native can't call the UniFFI-generated **Swift** API directly, and a
 * hand-written cinterop layer over the raw C FFI would have to reimplement
 * UniFFI's async continuation plumbing. So we invert the dependency: this
 * interface is exported into the ComposeApp framework as an Objective-C
 * protocol, the iOS host implements it in Swift against the generated
 * bindings, and hands the implementation to `MainViewController`.
 *
 * Everything is **callback-based on purpose.** Kotlin `suspend` functions in an
 * exported interface are awkward to conform to from Swift; a completion handler
 * is plain `@objc` and maps to a closure. [CoreBridgeRepository] converts these
 * back into `suspend` functions so the shared UI still sees the ordinary
 * `WalletRepository`.
 *
 * Each callback takes `(value, error)`. Exactly one is non-null. An error is
 * surfaced to the UI, never swallowed into a plausible-looking empty value.
 */
interface WalletCoreBridge {
    // ── synchronous: local seed-file reads, no network ──
    /** `configured`, `miningAddress`, `offer` — from the on-device seed file. */
    fun status(): BridgeStatus
    fun backupConfirmed(): Boolean
    fun confirmBackup()
    fun generate(): BridgeWalletSetup
    fun importSeed(mnemonic: String): BridgeWalletSetup
    fun revealSeed(): String
    /** Fixed BOLT11 amount in sats, or null when the payable is amountless. */
    fun payableAmountSats(payable: String): Long?

    // ── asynchronous: node + network ──
    fun initWallet(onDone: (String?, String?) -> Unit)
    fun createOffer(description: String, onDone: (String?, String?) -> Unit)
    fun nodeStatus(onDone: (BridgeNodeStatus?, String?) -> Unit)
    fun listPayments(limit: Int, onDone: (List<BridgeActivity>?, String?) -> Unit)
    fun createInvoice(amountSats: Long, description: String, onDone: (String?, String?) -> Unit)
    fun pay(payable: String, amountSats: Long, note: String?, onDone: (BridgePayment?, String?) -> Unit)
    fun poolStats(address: String, onDone: (BridgePoolStats?, String?) -> Unit)
    fun btcUsd(onDone: (Double) -> Unit)
}

// Plain data carriers across the boundary. Deliberately flat and using only
// types that survive the Kotlin→Objective-C export cleanly: no ULong (unsigned
// types don't export), no nullable primitives in constructor position beyond
// boxed Long?/Double?.

data class BridgeStatus(
    val configured: Boolean,
    val miningAddress: String?,
    val offer: String?,
)

data class BridgeWalletSetup(
    val mnemonic: String?,
    val miningAddress: String,
)

data class BridgeNodeStatus(
    val nodePk: String,
    val numChannels: Int,
    val numUsableChannels: Int,
    val lightningTotalSats: Long,
    val lightningSendableSats: Long,
    val onchainTotalSats: Long,
    val onchainTrustedSats: Long,
    val totalBalanceSats: Long,
)

data class BridgeActivity(
    val id: String,
    val direction: String,
    val rail: String,
    val amountSats: Long,
    val feeSats: Long,
    val status: String,
    val note: String?,
    val counterparty: String?,
    val finalizedAtMs: Long,
    val paymentHash: String?,
    val txid: String?,
    val isOcean: Boolean,
    /** -1 when absent (Objective-C export has no boxed optional here). */
    val blockHeight: Long,
    val preimage: String?,
    val offer: String?,
)

data class BridgePayment(
    val id: String,
    val amountSats: Long,
)

data class BridgePoolStats(
    val hashrate300s: Double,
    val hashrate3600s: Double,
    val hashrate10800s: Double,
    val hashrate86400s: Double,
    val activeWorkers: Int,
    val lastShareTs: Long,
    val unpaidSats: Long,
    val estPayoutNextBlockSats: Long,
    val estEarnNextBlockSats: Long,
    val totalPaidSats: Long,
    val lifetimeSats: Long,
    val tidesShares: Double,
    val poolTidesShares: Double,
    val sharePct: Double,
    val poolActiveUsers: Long,
    val poolActiveWorkers: Long,
    val networkDifficulty: Double,
    val addressUnknown: Boolean,
)
