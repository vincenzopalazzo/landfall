package xyz.ocean.mobile.data

// Mock data mirroring data.jsx so the app renders 1-1 with the design without a
// running node. `MockWalletRepository` serves this; `CoreWalletRepository` swaps
// in live data from the Rust core.
object Mock {
    // Fixed "now" anchor = design M_NOW (2026-06-05T14:10Z); relative times are
    // computed against it so the list labels match the handoff exactly.
    const val NOW_MS = 1_780_927_800_000L
    private fun ago(min: Long) = NOW_MS - min * 60_000L

    val pool = PoolStats(
        hashrate = 412.6, unit = "Th/s", workersOnline = 3, workersTotal = 4,
        poolHashrate = "1.24", poolUnit = "EH/s", sharePct = 0.033,
        blocksFound = 312, lastBlock = 897_180, lastBlockAgo = "8h ago",
        earned24h = 68_340, lifetimePaid = 2_418_000, rejectPct = 0.11,
    )

    val workers = listOf(
        Worker("antminer-s21", "Antminer S21", 235.1, true, 62, "12s ago"),
        Worker("s19xp-loft", "S19 XP", 141.0, true, 68, "8s ago"),
        Worker("s19j-garage", "S19j Pro", 36.5, true, 71, "21s ago"),
        Worker("s9-heater", "S9 (heater)", 0.0, false, null, "3h ago"),
    )

    val balances = Balances(channel = 1_284_503, capacity = 2_000_000, onchain = 3_812_000)

    val offer =
        "lno1pgsqvgnwgcg35z6ee2h3yczraddm72xrfua9uve2rlrm9deu7xyfzr5g3qfqu94yqsqd9hy6tq2qg3w0pmza4f4kqg83r9ftz29x0vqa8s2k4mflph6hq3kq7n0w5z2u4cdy3p9ka8w3hxu"

    val exchanges = listOf(
        Exchange("kraken", "Kraken", 0xFF5741D9, "bc1qkrkn7p2m4v8wq5r6t7y8u9i0a2s3d4f5g6h7k", false),
        Exchange("coinbase", "Coinbase", 0xFF0052FF, "bc1qcbse9x7k2m4p8v3wq5r6t7y8u9i0a2s3d4f5c", false),
        Exchange("river", "River", 0xFF0B7FD4, "bc1qrvr4t7y8u9i0a2s3d4f5g6h7k9x7k2m4p8v3w", false),
        Exchange("strike", "Strike", 0xFF000000, "node@strike.me", true),
    )

    val txs = listOf(
        Tx("t1", Dir.IN, Rail.LN, 47_820, TxStatus.MATURING, ago(12), "2026-06-05 13:58", "OCEAN",
            note = "Mining payout — maturing", offer = true,
            payerNote = "OCEAN payout · block 897,180 · ocean.xyz", noteMatch = true,
            hash = "b3c9f1a7e2d048a15c6b9f30e7a2d8c4f15e9b07a3d6c821f04e7b9a2c5d18e6f",
            fee = 0, maturing = true, mat = Maturity(6, MATURITY_TARGET, 897_180)),
        Tx("t2", Dir.IN, Rail.LN, 12_084, TxStatus.SETTLED, ago(296), "2026-06-05 09:14", "OCEAN",
            note = "Mining payout", offer = true,
            payerNote = "OCEAN payout · block 897,142 · ocean.xyz", noteMatch = true,
            hash = "7f2e1d9a4c6b8035e1f7a2d9c4b6083e5a1d7f9b2c4e6081a3d5f7b9c2e4068a",
            preimage = "a14c2e7f9b3d6082c5e1a7f4d9b206e3c8a5f102d7b4e69c3a8f5012e7d4b69c", fee = 0),
        Tx("t3", Dir.IN, Rail.LN, 25_000, TxStatus.SETTLED, ago(389), "2026-06-05 07:41",
            "satoshi@walletofsatoshi.com", note = "Tip — nice work on the pool",
            payerNote = "Thanks!",
            hash = "3a8f5012e7d4b69c3a8f5012e7d4b69c3a8f5012e7d4b69c3a8f5012e7d4b69c",
            preimage = "d7b4e69c3a8f5012e7d4b69c3a8f5012e7d4b69c3a8f5012e7d4b69c3a8f5012", fee = 0),
        Tx("t4", Dir.OUT, Rail.LN, 150_000, TxStatus.SETTLED, ago(960), "2026-06-04 22:10", "Kraken",
            note = "Withdraw to exchange",
            hash = "5e1d7f9b2c4e6081a3d5f7b9c2e4068a5e1d7f9b2c4e6081a3d5f7b9c2e4068a",
            preimage = "2c4e6081a3d5f7b9c2e4068a5e1d7f9b2c4e6081a3d5f7b9c2e4068a5e1d7f9b", fee = 41),
        Tx("t5", Dir.IN, Rail.LN, 9_732, TxStatus.SETTLED, ago(980), "2026-06-04 21:50", "OCEAN",
            note = "Mining payout", offer = true,
            payerNote = "OCEAN payout · block 897,098 · ocean.xyz", noteMatch = true,
            hash = "9c2e4068a5e1d7f9b2c4e6081a3d5f7b9c2e4068a5e1d7f9b2c4e6081a3d5f7b",
            preimage = "81a3d5f7b9c2e4068a5e1d7f9b2c4e6081a3d5f7b9c2e4068a5e1d7f9b2c4e60", fee = 0),
        Tx("t6", Dir.IN, Rail.ONCHAIN, 500_000, TxStatus.CONFIRMING, ago(1207), "2026-06-04 18:03",
            "On-chain deposit", note = "Incoming UTXO",
            txid = "e7a2d8c4f15e9b07a3d6c821f04e7b9a2c5d18e6fb3c9f1a7e2d048a15c6b9f3", conf = 2),
        Tx("t7", Dir.OUT, Rail.ONCHAIN, 1_000_000, TxStatus.SETTLED, ago(1368), "2026-06-04 15:22",
            "Cold storage", note = "To Coldcard (savings)",
            txid = "a3d6c821f04e7b9a2c5d18e6fb3c9f1a7e2d048a15c6b9f30e7a2d8c4f15e9b0", conf = 6, fee = 320),
        Tx("t8", Dir.IN, Rail.LN, 11_640, TxStatus.SETTLED, ago(1779), "2026-06-04 08:31", "OCEAN",
            note = "Mining payout", offer = true,
            payerNote = "OCEAN payout · block 897,001 · ocean.xyz", noteMatch = true,
            hash = "15c6b9f30e7a2d8c4f15e9b07a3d6c821f04e7b9a2c5d18e6fb3c9f1a7e2d048",
            preimage = "f04e7b9a2c5d18e6fb3c9f1a7e2d048a15c6b9f30e7a2d8c4f15e9b07a3d6c82", fee = 0),
        Tx("t9", Dir.IN, Rail.LN, 3_500, TxStatus.SETTLED, ago(1918), "2026-06-04 06:12", "fountain.fm",
            note = "Podcast streaming sats", payerNote = "Stream — Citadel Dispatch",
            hash = "c5d18e6fb3c9f1a7e2d048a15c6b9f30e7a2d8c4f15e9b07a3d6c821f04e7b9a",
            preimage = "0e7a2d8c4f15e9b07a3d6c821f04e7b9a2c5d18e6fb3c9f1a7e2d048a15c6b9f", fee = 0),
        Tx("t10", Dir.IN, Rail.LN, 8_905, TxStatus.SETTLED, ago(2583), "2026-06-03 19:07", "OCEAN",
            note = "Mining payout", offer = true,
            payerNote = "OCEAN payout · block 896,940 · ocean.xyz", noteMatch = true,
            hash = "a7e2d048a15c6b9f30e7a2d8c4f15e9b07a3d6c821f04e7b9a2c5d18e6fb3c9f",
            preimage = "30e7a2d8c4f15e9b07a3d6c821f04e7b9a2c5d18e6fb3c9f1a7e2d048a15c6b9", fee = 0),
        Tx("t11", Dir.OUT, Rail.LN, 21_000, TxStatus.FAILED, ago(2840), "2026-06-03 14:50", "alice@strike.me",
            note = "No route found — funds returned",
            hash = "f15e9b07a3d6c821f04e7b9a2c5d18e6fb3c9f1a7e2d048a15c6b9f30e7a2d8c", fee = 0),
        Tx("t12", Dir.IN, Rail.LN, 14_200, TxStatus.SETTLED, ago(3955), "2026-06-02 20:15", "Unknown sender",
            note = "Paid to your offer", offer = true, payerNote = "payout", noteMatch = false,
            hash = "048a15c6b9f30e7a2d8c4f15e9b07a3d6c821f04e7b9a2c5d18e6fb3c9f1a7e2",
            preimage = "821f04e7b9a2c5d18e6fb3c9f1a7e2d048a15c6b9f30e7a2d8c4f15e9b07a3d6", fee = 0),
    )
}
