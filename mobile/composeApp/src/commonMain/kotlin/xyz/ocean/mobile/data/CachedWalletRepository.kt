package xyz.ocean.mobile.data

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

/**
 * Caches the read side of a [WalletRepository] so moving between tabs doesn't
 * refetch what we already have.
 *
 * Without this, one lap of the tab bar cost 8 HTTP requests to ocean.xyz and 4
 * `nodeStatus()` calls, because every screen re-runs its `produceState` loader
 * on entering composition.
 *
 * Three properties matter more than the caching itself:
 *
 * 1. **TTLs are per-entry, matched to how fast the data actually moves** — see
 *    [Ttl]. A single global TTL would either hammer ocean.xyz or show a stale
 *    balance.
 * 2. **Mutations invalidate.** [pay] and [createInvoice] drop the node-derived
 *    entries; the seed operations drop everything. A stale balance after a send
 *    is the failure this class must not introduce.
 * 3. **Single-flight.** Two screens asking for the same key concurrently share
 *    one in-flight request rather than firing two.
 *
 * Writes are never cached, and neither are [revealSeed] or [payableAmountSats]
 * — a cached seed reveal would keep the phrase in memory longer than the one
 * call that needs it.
 */
class CachedWalletRepository(private val delegate: WalletRepository) : WalletRepository {

    /**
     * How long each kind of read stays fresh. Tuned to volatility, not uniform:
     * the node numbers are money and stay tight, the pool snapshot is four HTTP
     * calls against an API that only samples periodically.
     */
    private object Ttl {
        /** Seed-derived. Only changes through operations we invalidate on, so
         *  this is just a backstop against a missed invalidation. */
        const val STATUS = 5 * 60_000L
        const val BACKUP = 5 * 60_000L
        /** Balances. Short: a payment landing while you're on another tab shows
         *  up within this window of coming back. */
        const val NODE = 15_000L
        const val ACTIVITY = 20_000L
        /** Four ocean.xyz calls behind one entry. */
        const val POOL = 60_000L
        /** Mirrors the Rust `PriceClient` TTL; this just saves the FFI hop. */
        const val PRICE = 60_000L
    }

    private object Key {
        const val STATUS = "status"
        const val BACKUP = "backup"
        const val NODE = "node"
        const val ACTIVITY = "activity"
        const val POOL = "pool"
        const val PRICE = "price"
    }

    private class Entry(val value: Any?, val storedAtMs: Long)

    private val mutex = Mutex()
    private val entries = mutableMapOf<String, Entry>()
    private val inFlight = mutableMapOf<String, Deferred<Any?>>()

    /**
     * Owns in-flight loads so they survive the caller being cancelled.
     *
     * This is the case that matters: leaving a tab mid-fetch cancels that
     * screen's coroutine. If the load died with it, coming back would refetch —
     * exactly the behaviour we're removing. The load instead completes and
     * populates the cache for whoever asks next.
     */
    private val scope = CoroutineScope(SupervisorJob())

    override val nowMs: Long get() = delegate.nowMs

    // ── cached reads ──

    override suspend fun status(): WalletStatus =
        cached(Key.STATUS, Ttl.STATUS) { delegate.status() }

    override suspend fun nodeInfo(): NodeInfo =
        cached(Key.NODE, Ttl.NODE) { delegate.nodeInfo() }

    override suspend fun activity(): List<Tx> =
        cached(Key.ACTIVITY, Ttl.ACTIVITY) { delegate.activity() }

    override suspend fun pool(): PoolStats? =
        cached(Key.POOL, Ttl.POOL) { delegate.pool() }

    override suspend fun btcUsd(): Double =
        cached(Key.PRICE, Ttl.PRICE) { delegate.btcUsd() }

    override suspend fun isBackupConfirmed(): Boolean =
        cached(Key.BACKUP, Ttl.BACKUP) { delegate.isBackupConfirmed() }

    // `offer()`, `miningAddress()`, `isWalletConfigured()` and `balances()` are
    // interface defaults over `status()` / `nodeInfo()`, so they ride the same
    // two cache entries. Nothing to override.

    // ── uncached passthrough ──

    override suspend fun revealSeed(): String = delegate.revealSeed()

    override suspend fun payableAmountSats(payable: String): Long? =
        delegate.payableAmountSats(payable)

    // ── mutations: perform, then invalidate what they changed ──

    override suspend fun pay(payable: String, amountSats: Long?, note: String?): PaymentResult {
        val result = delegate.pay(payable, amountSats, note)
        // Balance dropped and a row appeared. Serving the pre-send balance here
        // would be the worst bug this class could ship.
        invalidate(Key.NODE, Key.ACTIVITY)
        return result
    }

    override suspend fun createInvoice(amountSats: Long?, description: String?): String {
        val invoice = delegate.createInvoice(amountSats, description)
        // No balance change yet, but a pending inbound row can appear.
        invalidate(Key.ACTIVITY)
        return invoice
    }

    override suspend fun generateWallet(): WalletSetup =
        delegate.generateWallet().also { invalidateAll() }

    override suspend fun restoreWallet(mnemonic: String): WalletSetup =
        delegate.restoreWallet(mnemonic).also { invalidateAll() }

    override suspend fun completeWalletSetup() {
        delegate.completeWalletSetup()
        // Provisioning derives the mining address and creates the offer.
        invalidateAll()
    }

    override suspend fun confirmBackup() {
        delegate.confirmBackup()
        invalidate(Key.BACKUP)
    }

    override suspend fun refresh() = invalidateAll()

    // ── machinery ──

    /**
     * Return the cached value for [key] when still within [ttlMs], otherwise
     * load it — joining an existing in-flight load rather than starting a
     * second one.
     *
     * A failed load stores nothing, so the next caller retries instead of
     * caching an error.
     */
    @Suppress("UNCHECKED_CAST")
    private suspend fun <T> cached(key: String, ttlMs: Long, load: suspend () -> T): T {
        val job: Deferred<Any?> = mutex.withLock {
            // Check the Entry, not its value: `pool()` legitimately caches null
            // (no mining address yet) and that must count as a hit.
            entries[key]?.let { hit ->
                if (delegate.nowMs - hit.storedAtMs < ttlMs) return hit.value as T
            }
            inFlight.getOrPut(key) {
                // Dispatched, so the body can't run inline while we hold the
                // mutex; it takes the lock itself once we've released it.
                scope.async {
                    try {
                        val value = load()
                        mutex.withLock { entries[key] = Entry(value, delegate.nowMs) }
                        value
                    } finally {
                        // Always clear, so a failure doesn't pin a dead Deferred.
                        mutex.withLock { inFlight.remove(key) }
                    }
                }
            }
        }
        return job.await() as T
    }

    private suspend fun invalidate(vararg keys: String) = mutex.withLock {
        keys.forEach { entries.remove(it) }
    }

    private suspend fun invalidateAll() = mutex.withLock {
        entries.clear()
    }
}

/** Wrap this repository so screen switches serve cached reads. */
fun WalletRepository.cached(): WalletRepository = CachedWalletRepository(this)
