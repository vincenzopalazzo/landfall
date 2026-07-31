package xyz.ocean.mobile.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

/**
 * Regression tests for two bugs found by running against a real wallet.
 *
 * The numbers below are the actual readings from that node and from
 * `api.ocean.xyz` at the time: the app reported a 5,553 sat balance for a
 * 6,586 sat wallet, and "0 H/s" for a rig averaging 312 Gh/s over 3 hours.
 */
class BalanceAndHashrateTest {

    private fun node(
        sendable: Long = 5_553,
        lightningTotal: Long = 6_586,
        onchainTrusted: Long = 0,
        onchainTotal: Long = 0,
        total: Long = 6_586,
    ) = NodeInfo(
        nodePk = "03pk",
        channels = 1,
        usableChannels = 1,
        lightningTotalSats = lightningTotal,
        lightningSendableSats = sendable,
        onchainTotalSats = onchainTotal,
        onchainTrustedSats = onchainTrusted,
        totalBalanceSats = total,
    )

    @Test
    fun total_is_the_nodes_own_figure_not_sendable_plus_onchain() {
        // The reserve (1,033 sats here) is owned but not spendable. Deriving
        // the total as `sendable + onchain` dropped it entirely.
        val b = node().toBalances()
        assertEquals(6_586, b.total, "total must include the channel reserve")
        assertEquals(5_553, b.channel, "channel is still what can be sent now")
        assertEquals(1_033, b.reserved)
    }

    @Test
    fun nothing_is_reserved_when_everything_is_sendable() {
        val b = node(sendable = 6_586, total = 6_586).toBalances()
        assertEquals(0, b.reserved)
        assertEquals(b.total, b.channel)
    }

    @Test
    fun reserved_never_goes_negative() {
        // Defensive: a node reporting sendable above the channel total must not
        // produce a negative "reserved" that would render as nonsense.
        val b = node(sendable = 9_000, lightningTotal = 6_586).toBalances()
        assertEquals(0, b.reserved)
    }

    @Test
    fun unconfirmed_on_chain_is_not_counted_as_channel_reserve() {
        // `total_balance_sats` includes unconfirmed on-chain but `onchain` is
        // the trusted figure, so deriving reserve by subtraction would label
        // someone's unconfirmed funds as channel reserve. It is the Lightning
        // gap and nothing else.
        val b = node(
            sendable = 5_553, lightningTotal = 6_586,
            onchainTrusted = 1_000, onchainTotal = 4_000, total = 10_586,
        ).toBalances()
        assertEquals(1_033, b.reserved, "reserve is lightningTotal - sendable, only")
    }

    @Test
    fun on_chain_funds_count_toward_the_total() {
        val b = node(onchainTrusted = 2_000, onchainTotal = 2_000, total = 8_586).toBalances()
        assertEquals(8_586, b.total)
        assertEquals(2_000, b.onchain)
        assertEquals(1_033, b.reserved)
    }

    private fun pool(
        h300: Double = 0.0,
        h3600: Double = 0.0,
        h10800: Double = 0.0,
        h86400: Double = 0.0,
    ) = PoolStats(
        hashrate300s = h300, hashrate3600s = h3600,
        hashrate10800s = h10800, hashrate86400s = h86400,
        activeWorkers = 0, lastShareTs = 0, unpaidSats = 0,
        estPayoutNextBlockSats = 0, estEarnNextBlockSats = 0,
        totalPaidSats = 0, lifetimeSats = 0, tidesShares = 0.0,
        poolTidesShares = 0.0, sharePct = 0.0, poolActiveUsers = 0,
        poolActiveWorkers = 0, networkDifficulty = 0.0, addressUnknown = false,
    )

    @Test
    fun hashrate_falls_back_to_a_longer_window_when_the_miner_paused() {
        // The exact reading that showed "0 H/s": idle for 5m and 1h, but
        // 312 Gh/s over 3h and 899 Gh/s over 24h.
        val best = pool(h10800 = 312_749_974_123.0, h86400 = 899_156_175_603.0).bestHashrate()
        assertEquals(312_749_974_123.0, best?.first)
        assertEquals("3 HR", best?.second)
    }

    @Test
    fun the_shortest_window_with_a_reading_wins() {
        val best = pool(
            h300 = 1_000.0, h3600 = 2_000.0, h10800 = 3_000.0, h86400 = 4_000.0,
        ).bestHashrate()
        assertEquals(1_000.0, best?.first)
        assertEquals("5 MIN", best?.second)
    }

    @Test
    fun falls_through_each_window_in_order() {
        assertEquals("1 HR", pool(h3600 = 5.0, h10800 = 6.0).bestHashrate()?.second)
        assertEquals("24 HR", pool(h86400 = 7.0).bestHashrate()?.second)
    }

    @Test
    fun all_windows_zero_means_genuinely_not_mining() {
        assertNull(pool().bestHashrate(), "null is the honest 'not mining' state")
    }
}
