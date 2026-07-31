package xyz.ocean.mobile.data

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.async
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * A [WalletRepository] that counts calls and lets the test drive the clock.
 *
 * `nowMs` is what `CachedWalletRepository` uses for TTL comparisons, so moving
 * `clock` forward is how these tests expire entries without sleeping.
 */
private class FakeRepo : WalletRepository {
    var clock = 0L
    override val nowMs: Long get() = clock

    var statusCalls = 0
    var nodeCalls = 0
    var activityCalls = 0
    var poolCalls = 0
    var priceCalls = 0
    var backupCalls = 0

    var statusValue = WalletStatus(configured = true, miningAddress = "bc1qtest", offer = "lno1test")
    var sendable = 1_000L
    var activityRows = listOf(tx("a"))
    /** Set to make the next `nodeInfo()` throw, to test failures aren't cached. */
    var nodeError: String? = null
    /** Gates `nodeInfo()` so a test can hold two callers in flight at once. */
    var nodeGate: CompletableDeferred<Unit>? = null

    override suspend fun status(): WalletStatus {
        statusCalls++
        return statusValue
    }

    override suspend fun nodeInfo(): NodeInfo {
        nodeGate?.await()
        nodeCalls++
        nodeError?.let { throw IllegalStateException(it) }
        return NodeInfo(
            nodePk = "03pk",
            channels = 2,
            usableChannels = 1,
            lightningTotalSats = 5_000,
            lightningSendableSats = sendable,
            onchainTotalSats = 300,
            onchainTrustedSats = 200,
            totalBalanceSats = 5_200,
        )
    }

    override suspend fun activity(): List<Tx> {
        activityCalls++
        return activityRows
    }

    override suspend fun pool(): PoolStats? {
        poolCalls++
        return null
    }

    override suspend fun btcUsd(): Double {
        priceCalls++
        return 96_000.0
    }

    override suspend fun isBackupConfirmed(): Boolean {
        backupCalls++
        return true
    }

    // Unused by these tests.
    override suspend fun generateWallet() = WalletSetup("m", "bc1q")
    override suspend fun restoreWallet(mnemonic: String) = WalletSetup(null, "bc1q")
    override suspend fun revealSeed() = "phrase"
    override suspend fun completeWalletSetup() = Unit
    override suspend fun confirmBackup() = Unit
    override suspend fun payableAmountSats(payable: String): Long? = null
    override suspend fun createInvoice(amountSats: Long?, description: String?) = "lnbc1"
    override suspend fun pay(payable: String, amountSats: Long?, note: String?) =
        PaymentResult("pay-1", amountSats ?: 0L)
}

private fun tx(id: String) = Tx(
    id = id, dir = Dir.IN, rail = Rail.LN, amt = 10, status = TxStatus.SETTLED,
    tsMs = 0, party = "someone",
)

class CachedWalletRepositoryTest {

    @Test
    fun repeated_reads_within_ttl_hit_the_cache() = runTest {
        val fake = FakeRepo()
        val cache = CachedWalletRepository(fake)

        repeat(5) { cache.nodeInfo() }
        repeat(5) { cache.activity() }
        repeat(5) { cache.pool() }
        repeat(5) { cache.btcUsd() }

        assertEquals(1, fake.nodeCalls)
        assertEquals(1, fake.activityCalls)
        assertEquals(1, fake.poolCalls)
        assertEquals(1, fake.priceCalls)
    }

    /**
     * The whole point of the change: a lap of the tab bar must not refetch.
     * Pool reads pool+balances, Wallet reads balances+activity, Node reads
     * nodeInfo+offer+backup — and coming back to Pool costs nothing.
     */
    @Test
    fun a_full_tab_lap_costs_one_call_per_source() = runTest {
        val fake = FakeRepo()
        val cache = CachedWalletRepository(fake)

        // Pool
        cache.pool(); cache.balances()
        // Wallet
        cache.balances(); cache.activity()
        // Node
        cache.nodeInfo(); cache.offer(); cache.isBackupConfirmed()
        // back to Pool
        cache.pool(); cache.balances()

        assertEquals(1, fake.poolCalls)
        assertEquals(1, fake.nodeCalls, "balances() and nodeInfo() must share one node call")
        assertEquals(1, fake.activityCalls)
        assertEquals(1, fake.statusCalls)
        assertEquals(1, fake.backupCalls)
    }

    @Test
    fun balances_and_node_info_share_one_entry() = runTest {
        val fake = FakeRepo()
        val cache = CachedWalletRepository(fake)

        val balances = cache.balances()
        val info = cache.nodeInfo()

        assertEquals(1, fake.nodeCalls)
        assertEquals(1_000L, balances.channel)
        assertEquals(200L, balances.onchain)
        assertEquals("03pk", info.nodePk)
    }

    @Test
    fun offer_and_mining_address_share_one_status_call() = runTest {
        val fake = FakeRepo()
        val cache = CachedWalletRepository(fake)

        assertEquals("lno1test", cache.offer())
        assertEquals("bc1qtest", cache.miningAddress())
        assertTrue(cache.isWalletConfigured())

        assertEquals(1, fake.statusCalls)
    }

    @Test
    fun entries_refetch_once_the_ttl_expires() = runTest {
        val fake = FakeRepo()
        val cache = CachedWalletRepository(fake)

        cache.nodeInfo()
        assertEquals(1, fake.nodeCalls)

        // Just inside the 15s node TTL.
        fake.clock = 14_999
        cache.nodeInfo()
        assertEquals(1, fake.nodeCalls)

        // Past it.
        fake.clock = 15_001
        cache.nodeInfo()
        assertEquals(2, fake.nodeCalls)
    }

    @Test
    fun pool_holds_longer_than_the_node_balance() = runTest {
        val fake = FakeRepo()
        val cache = CachedWalletRepository(fake)

        cache.pool(); cache.nodeInfo()
        // 30s: past the node TTL (15s), inside the pool TTL (60s). Four
        // ocean.xyz calls must not fire just because the balance went stale.
        fake.clock = 30_000
        cache.pool(); cache.nodeInfo()

        assertEquals(1, fake.poolCalls)
        assertEquals(2, fake.nodeCalls)
    }

    @Test
    fun pay_invalidates_the_balance_and_the_activity_list() = runTest {
        val fake = FakeRepo()
        val cache = CachedWalletRepository(fake)

        assertEquals(1_000L, cache.balances().channel)
        cache.activity()
        assertEquals(1, fake.nodeCalls)
        assertEquals(1, fake.activityCalls)

        // The node now reports a lower balance and an extra row.
        fake.sendable = 400
        fake.activityRows = listOf(tx("a"), tx("b"))
        cache.pay("lnbc1", 600, null)

        // Must not serve the pre-send balance, even though the TTL hasn't passed.
        assertEquals(400L, cache.balances().channel)
        assertEquals(2, cache.activity().size)
        assertEquals(2, fake.nodeCalls)
        assertEquals(2, fake.activityCalls)
    }

    @Test
    fun pay_does_not_invalidate_unrelated_entries() = runTest {
        val fake = FakeRepo()
        val cache = CachedWalletRepository(fake)

        cache.pool(); cache.offer()
        cache.pay("lnbc1", 10, null)
        cache.pool(); cache.offer()

        // Sending does not change the mining stats or the offer.
        assertEquals(1, fake.poolCalls)
        assertEquals(1, fake.statusCalls)
    }

    @Test
    fun create_invoice_invalidates_activity_only() = runTest {
        val fake = FakeRepo()
        val cache = CachedWalletRepository(fake)

        cache.activity(); cache.nodeInfo()
        cache.createInvoice(1_000, "test")
        cache.activity(); cache.nodeInfo()

        assertEquals(2, fake.activityCalls, "a pending inbound row can appear")
        assertEquals(1, fake.nodeCalls, "creating an invoice moves no funds")
    }

    @Test
    fun confirm_backup_invalidates_the_backup_flag() = runTest {
        val fake = FakeRepo()
        val cache = CachedWalletRepository(fake)

        cache.isBackupConfirmed()
        cache.confirmBackup()
        cache.isBackupConfirmed()

        assertEquals(2, fake.backupCalls)
    }

    @Test
    fun refresh_drops_everything() = runTest {
        val fake = FakeRepo()
        val cache = CachedWalletRepository(fake)

        cache.pool(); cache.nodeInfo(); cache.activity(); cache.offer(); cache.btcUsd()
        cache.refresh()
        cache.pool(); cache.nodeInfo(); cache.activity(); cache.offer(); cache.btcUsd()

        assertEquals(2, fake.poolCalls)
        assertEquals(2, fake.nodeCalls)
        assertEquals(2, fake.activityCalls)
        assertEquals(2, fake.statusCalls)
        assertEquals(2, fake.priceCalls)
    }

    @Test
    fun concurrent_callers_share_one_request() = runTest {
        val fake = FakeRepo()
        val gate = CompletableDeferred<Unit>()
        fake.nodeGate = gate
        val cache = CachedWalletRepository(fake)

        // Three screens ask at once while the load is still in flight.
        val a = async { cache.nodeInfo() }
        val b = async { cache.balances() }
        val c = async { cache.nodeInfo() }
        gate.complete(Unit)
        a.await(); b.await(); c.await()

        assertEquals(1, fake.nodeCalls, "single-flight: one delegate call for three callers")
    }

    @Test
    fun a_failed_load_is_not_cached() = runTest {
        val fake = FakeRepo()
        val cache = CachedWalletRepository(fake)

        fake.nodeError = "node unreachable"
        assertFailsWith<IllegalStateException> { cache.nodeInfo() }
        assertEquals(1, fake.nodeCalls)

        // The next caller must retry rather than get a cached error.
        fake.nodeError = null
        assertEquals("03pk", cache.nodeInfo().nodePk)
        assertEquals(2, fake.nodeCalls)
    }

    @Test
    fun seed_reads_and_payment_helpers_are_never_cached() = runTest {
        val fake = FakeRepo()
        val cache = CachedWalletRepository(fake)

        // A cached seed reveal would keep the phrase in memory past the one call
        // that needs it.
        repeat(3) { assertEquals("phrase", cache.revealSeed()) }
        assertNull(cache.payableAmountSats("lnbc1"))
    }
}
