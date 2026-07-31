package xyz.ocean.mobile.data

/**
 * The shortest hashrate window that actually has a reading, with its label.
 *
 * OCEAN reports 0 for a window the miner was idle through, so pinning the
 * headline to the 5-minute average showed "0 H/s" for a rig that had simply
 * paused — while the 3h average was 312 Gh/s. The web dashboard cascades
 * through the windows and relabels (`StatsGrid.svelte`); this matches it, so
 * the two cannot disagree about whether someone is mining.
 *
 * Null when every window is zero — genuinely not mining.
 */
fun PoolStats.bestHashrate(): Pair<Double, String>? = when {
    hashrate300s > 0.0 -> hashrate300s to "5 MIN"
    hashrate3600s > 0.0 -> hashrate3600s to "1 HR"
    hashrate10800s > 0.0 -> hashrate10800s to "3 HR"
    hashrate86400s > 0.0 -> hashrate86400s to "24 HR"
    else -> null
}
