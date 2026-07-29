package xyz.ocean.mobile.data

// UI models mirroring the design's data.jsx shapes. These are what the screens
// render; the repository maps either mock data or the Rust core's UniFFI records
// into them.

enum class Dir { IN, OUT }
enum class Rail { LN, ONCHAIN }
enum class TxStatus { SETTLED, PENDING, CONFIRMING, MATURING, FAILED }

data class Maturity(
    val confs: Int,
    val target: Int,
    val block: Long,
)

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
    val conf: Int? = null,
    val maturing: Boolean = false,
    val mat: Maturity? = null,
) {
    // OCEAN payout: paid to our registered offer AND the payer note matches.
    val isOcean: Boolean get() = dir == Dir.IN && offer && noteMatch
}

data class Worker(
    val id: String,
    val model: String,
    val hr: Double,            // Th/s
    val online: Boolean,
    val temp: Int? = null,
    val ago: String,
)

data class PoolStats(
    val hashrate: Double,
    val unit: String,
    val workersOnline: Int,
    val workersTotal: Int,
    val poolHashrate: String,
    val poolUnit: String,
    val sharePct: Double,
    val blocksFound: Long,
    val lastBlock: Long,
    val lastBlockAgo: String,
    val earned24h: Long,
    val lifetimePaid: Long,
    val rejectPct: Double,
)

data class Balances(
    val channel: Long,
    val capacity: Long,
    val onchain: Long,
) {
    val total: Long get() = channel + onchain
}

data class Exchange(
    val id: String,
    val name: String,
    val colorHex: Long,
    val addr: String,
    val lightning: Boolean,
)
