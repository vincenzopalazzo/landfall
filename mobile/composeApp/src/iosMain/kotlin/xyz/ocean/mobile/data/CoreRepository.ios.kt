package xyz.ocean.mobile.data

import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.suspendCancellableCoroutine

// iOS core binding.
//
// The Rust core is reached through [WalletCoreBridge], implemented in Swift by
// the iOS host against the UniFFI-generated Swift API (see
// iosApp/iosApp/CoreBridge.swift). This file is the adapter: it turns the
// bridge's completion handlers back into the `suspend` functions the shared UI
// expects, so `WalletRepository` looks identical on both platforms.
//
// There is no fixture fallback here, on either platform. If the bridge is
// missing the app shows an error — it never renders a plausible fake wallet.

/**
 * Set by `MainViewController(bridge:)` before the first composition. The
 * `appDataDir` argument is unused on iOS: the Swift side already rooted the
 * core at the app's Documents directory when it constructed the bridge.
 */
internal var installedBridge: WalletCoreBridge? = null

actual fun createCoreRepository(appDataDir: String): WalletRepository {
    val bridge = installedBridge
        ?: throw IllegalStateException(
            "The iOS host did not install a wallet core bridge. " +
                "MainViewController(bridge:) must be called with a CoreBridge instance.",
        )
    return CoreBridgeRepository(bridge)
}

/** Sentinel for "no value" across the Objective-C boundary, which has no Long?. */
private const val NONE = -1L

private class CoreBridgeRepository(private val bridge: WalletCoreBridge) : WalletRepository {
    override val nowMs: Long get() = currentTimeMillis()

    // One call serves offer / miningAddress / isWalletConfigured — those are
    // interface defaults over this.
    //
    // Throwing on `error` is what keeps a failed read out of the cache: the
    // cache stores results, not exceptions, so a transient seed-file error
    // retries instead of being served as "no address, no offer" for the TTL.
    override suspend fun status(): WalletStatus = bridge.status().let {
        it.error?.let { msg -> throw CoreBridgeException(msg) }
        WalletStatus(
            configured = it.configured,
            miningAddress = it.miningAddress,
            offer = it.offer,
        )
    }

    override suspend fun isBackupConfirmed(): Boolean = bridge.backupConfirmed()

    override suspend fun confirmBackup() {
        bridge.confirmBackup()?.let { throw CoreBridgeException(it) }
    }

    override suspend fun revealSeed(): String = bridge.revealSeed().let {
        it.error?.let { msg -> throw CoreBridgeException(msg) }
        it.mnemonic ?: throw CoreBridgeException("the wallet core returned no recovery phrase")
    }

    override suspend fun generateWallet(): WalletSetup = bridge.generate().let {
        it.error?.let { msg -> throw CoreBridgeException(msg) }
        WalletSetup(it.mnemonic, it.miningAddress)
    }

    override suspend fun restoreWallet(mnemonic: String): WalletSetup {
        val result = bridge.importSeed(mnemonic)
        result.error?.let { throw CoreBridgeException(it) }
        completeWalletSetup()
        // A restored phrase is already in the user's hands; nothing to write down.
        bridge.confirmBackup()
        return WalletSetup(null, result.miningAddress)
    }

    override suspend fun completeWalletSetup() {
        awaiting { cb -> bridge.initWallet(cb) }
        if (status().offer == null) {
            awaiting { cb -> bridge.createOffer("OCEAN Lightning mobile", cb) }
        }
    }

    override suspend fun payableAmountSats(payable: String): Long? =
        bridge.payableAmountSats(payable)

    override suspend fun createInvoice(amountSats: Long?, description: String?): String =
        awaiting { cb -> bridge.createInvoice(amountSats ?: NONE, description ?: "", cb) }

    override suspend fun pay(payable: String, amountSats: Long?, note: String?): PaymentResult =
        awaiting { cb -> bridge.pay(payable, amountSats ?: NONE, note, cb) }
            .let { PaymentResult(it.id, it.amountSats) }

    override suspend fun pool(): PoolStats? {
        val address = status().miningAddress ?: return null
        return awaiting<BridgePoolStats> { cb -> bridge.poolStats(address, cb) }.toUi()
    }

    // `balances()` is an interface default over this, so the Wallet and Node
    // screens share one `nodeStatus()` round-trip instead of making two.
    override suspend fun nodeInfo(): NodeInfo {
        val n = awaiting<BridgeNodeStatus> { cb -> bridge.nodeStatus(cb) }
        return NodeInfo(
            nodePk = n.nodePk,
            channels = n.numChannels,
            usableChannels = n.numUsableChannels,
            lightningTotalSats = n.lightningTotalSats,
            lightningSendableSats = n.lightningSendableSats,
            onchainTotalSats = n.onchainTotalSats,
            onchainTrustedSats = n.onchainTrustedSats,
            totalBalanceSats = n.totalBalanceSats,
        )
    }

    override suspend fun activity(): List<Tx> =
        awaiting<List<BridgeActivity>> { cb -> bridge.listPayments(200, cb) }.map { it.toTx() }

    // The price never fails: 0.0 means "unavailable", and the UI shows sats.
    override suspend fun btcUsd(): Double =
        suspendCancellableCoroutine { cont -> bridge.btcUsd { cont.resume(it) } }
}

/**
 * Bridge a `(value, error)` completion handler into a `suspend` call.
 *
 * A non-null error becomes a thrown exception so the existing `runCatching`
 * paths in the UI surface it; a null value with a null error would be a bridge
 * bug, and is reported as one rather than silently becoming a default.
 */
private suspend fun <T : Any> awaiting(
    start: ((T?, String?) -> Unit) -> Unit,
): T = suspendCancellableCoroutine { cont ->
    start { value, error ->
        when {
            error != null -> cont.resumeWithException(CoreBridgeException(error))
            value != null -> cont.resume(value)
            else -> cont.resumeWithException(
                CoreBridgeException("wallet core returned neither a result nor an error"),
            )
        }
    }
}

class CoreBridgeException(message: String) : Exception(message)

private fun BridgePoolStats.toUi(): PoolStats = PoolStats(
    hashrate300s = hashrate300s,
    hashrate3600s = hashrate3600s,
    hashrate10800s = hashrate10800s,
    hashrate86400s = hashrate86400s,
    activeWorkers = activeWorkers,
    lastShareTs = lastShareTs,
    unpaidSats = unpaidSats,
    estPayoutNextBlockSats = estPayoutNextBlockSats,
    estEarnNextBlockSats = estEarnNextBlockSats,
    totalPaidSats = totalPaidSats,
    lifetimeSats = lifetimeSats,
    tidesShares = tidesShares,
    poolTidesShares = poolTidesShares,
    sharePct = sharePct,
    poolActiveUsers = poolActiveUsers,
    poolActiveWorkers = poolActiveWorkers,
    networkDifficulty = networkDifficulty,
    addressUnknown = addressUnknown,
)

private fun BridgeActivity.toTx(): Tx = Tx(
    id = id,
    dir = if (direction == "in") Dir.IN else Dir.OUT,
    rail = if (rail == "ln") Rail.LN else Rail.ONCHAIN,
    amt = amountSats,
    status = when (status) {
        "settled" -> TxStatus.SETTLED
        "failed" -> TxStatus.FAILED
        else -> TxStatus.PENDING
    },
    tsMs = finalizedAtMs,
    tsIso = isoUtc(finalizedAtMs),
    party = counterparty ?: (if (isOcean) "OCEAN-format payment" else "Unknown"),
    note = note,
    offer = offer != null || isOcean,
    payerNote = note,
    noteMatch = isOcean,
    fee = feeSats,
    hash = paymentHash,
    preimage = preimage,
    txid = txid,
    oceanBlockHeight = blockHeight.takeIf { it >= 0 },
)
