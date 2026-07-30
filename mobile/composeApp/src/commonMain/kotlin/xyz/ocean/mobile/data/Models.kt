package xyz.ocean.mobile.data

// UI models. Every field here must be sourceable from something real — the
// Rust core (node balances, payments, seed state) or OCEAN's public API via
// `OceanlnCore.poolStats`. If a screen wants a number no backend can produce,
// the number does not belong in this file.

enum class Dir { IN, OUT }
enum class Rail { LN, ONCHAIN }
// The core reports exactly these three (lexe_wallet::activity_from maps
// Completed/Failed/_ → settled/failed/pending). No other state is reachable.
enum class TxStatus { SETTLED, PENDING, FAILED }

data class Tx(
    val id: String,
    val dir: Dir,
    val rail: Rail,
    val amt: Long?,             // sats; null = pending/unknown
    val status: TxStatus,
    val tsMs: Long,             // epoch millis (for relative time)
    val tsIso: String = "",     // absolute label for the detail sheet
    val party: String,
    val note: String? = null,
    val offer: Boolean = false,
    val payerNote: String? = null,
    val noteMatch: Boolean = false,
    val fee: Long? = null,
    val hash: String? = null,
    val preimage: String? = null,
    val txid: String? = null,
    // Block height parsed out of the OCEAN payer note — i.e. the block whose
    // reward this payout is for. Present only on OCEAN-format payments, and
    // NOT a chain confirmation depth (the app has no tip to measure against).
    val oceanBlockHeight: Long? = null,
) {
    // OCEAN-format candidate: paid to an offer with the expected public note
    // format. This is classification only, not cryptographic authentication.
    val isOcean: Boolean get() = dir == Dir.IN && offer && noteMatch
}

/**
 * Live mining stats, 1-1 with the Rust `PoolStats` record — which in turn is
 * 1-1 with what `api.ocean.xyz/v1` actually returns.
 *
 * Note what is *not* here: pool-wide hashrate, blocks-found, last-block
 * height/age, reject rate, and a per-worker list. OCEAN's public API exposes
 * none of them, so the UI does not show them.
 */
data class PoolStats(
    // hashes/sec per window
    val hashrate300s: Double,
    val hashrate3600s: Double,
    val hashrate10800s: Double,
    val hashrate86400s: Double,
    /** Live worker count. There is no "total configured" figure upstream. */
    val activeWorkers: Int,
    /** Unix seconds of the most recent accepted share; 0 = never. */
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
    /** Address has never mined here — a real empty state, not a fetch error. */
    val addressUnknown: Boolean,
)

/** Seed-derived wallet state — mirrors the core's `StatusResp`. */
data class WalletStatus(
    val configured: Boolean,
    val miningAddress: String?,
    val offer: String?,
)

/**
 * Identity, channel shape and balances of the in-process Lexe node — the whole
 * of one `nodeStatus()` call.
 *
 * [Balances] is derived from this rather than fetched separately: both came from
 * the same core call, so keeping them as one value means one round-trip and one
 * cache entry instead of two of each.
 */
data class NodeInfo(
    val nodePk: String,
    val channels: Int,
    val usableChannels: Int,
    val lightningTotalSats: Long,
    val lightningSendableSats: Long,
    val onchainTotalSats: Long,
    val onchainTrustedSats: Long,
    val totalBalanceSats: Long,
) {
    fun toBalances(): Balances = Balances(
        channel = lightningSendableSats,
        // Floored at 1 so the channel-capacity progress bar can't divide by zero.
        capacity = maxOf(lightningTotalSats, 1L),
        onchain = onchainTrustedSats,
        // The node's own figure. Deriving this as `sendable + onchain` silently
        // dropped the channel reserve — funds you own but cannot send right now
        // — so a 6,586 sat wallet reported 5,553.
        total = totalBalanceSats,
    )
}

/**
 * What the wallet holds.
 *
 * [channel] is what Lightning can send *right now*; [total] is everything the
 * node owns, including the channel reserve — yours, but not currently
 * spendable. Conflating the two understates the balance.
 */
data class Balances(
    /** Lightning outbound liquidity — the spendable figure. */
    val channel: Long,
    /** Total channel capacity, for the liquidity bar. */
    val capacity: Long,
    val onchain: Long,
    /** Everything the node owns, reserve included. */
    val total: Long,
) {
    /** Held but not spendable right now (channel reserve). */
    val reserved: Long get() = (total - channel - onchain).coerceAtLeast(0L)
}
