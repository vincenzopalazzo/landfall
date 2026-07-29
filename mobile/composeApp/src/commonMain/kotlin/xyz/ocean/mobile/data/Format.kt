package xyz.ocean.mobile.data

import kotlin.math.abs
import kotlin.math.floor
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToLong

// Formatting helpers — 1-1 with data.jsx (mCommas, mUsd, mFmtUsd, mBtc, mShort,
// matPct, matEta). USD rate is a mock, matching the design.
const val USD_PER_BTC = 96_200.0
const val MATURITY_TARGET = 100

fun commas(n: Long): String {
    val s = abs(n).toString()
    val sb = StringBuilder()
    for ((i, c) in s.withIndex()) {
        if (i > 0 && (s.length - i) % 3 == 0) sb.append(',')
        sb.append(c)
    }
    return (if (n < 0) "-" else "") + sb.toString()
}

fun usd(sats: Long): Double = (sats / 1e8) * USD_PER_BTC

fun fmtUsd(v: Double): String {
    val cents = (v * 100).roundToLong()
    val whole = cents / 100
    val frac = (abs(cents) % 100).toString().padStart(2, '0')
    return "$" + commas(whole) + "." + frac
}

fun btc(sats: Long): String {
    val s = (sats / 1e8)
    var out = ((s * 1e8).roundToLong() / 1e8).toString()
    // strip trailing zeros / dot
    if (out.contains('.')) out = out.trimEnd('0').trimEnd('.')
    return out
}

fun short(s: String?, a: Int = 10, b: Int = 8): String =
    if (s == null || s.length <= a + b) (s ?: "—")
    else s.take(a) + "…" + s.takeLast(b)

fun matPct(m: Maturity): Float =
    max(3f, min(100f, (m.confs.toFloat() / m.target) * 100f))

fun matEta(m: Maturity): String {
    val mins = max(0, m.target - m.confs) * 10
    if (mins <= 0) return "ready now"
    val h = mins / 60
    val mm = mins % 60
    return if (h > 0) "~${h}h" + (if (mm != 0) " ${mm}m" else "") else "~${mm}m"
}

// relative time (mRel): minutes/hours/days ago, against a supplied "now".
fun rel(tsMs: Long, nowMs: Long): String {
    val m = ((nowMs - tsMs) / 60_000.0).roundToLong()
    if (m < 1) return "just now"
    if (m < 60) return "${m}m ago"
    val h = (m / 60.0).roundToLong()
    if (h < 24) return "${h}h ago"
    return "${(h / 24.0).roundToLong()}d ago"
}

// signed amount honoring the unit toggle
fun amt(sats: Long?, usdUnit: Boolean, dir: Dir?): String {
    if (sats == null) return "—"
    val sign = when (dir) { Dir.IN -> "+"; Dir.OUT -> "−"; else -> "" }
    return sign + if (usdUnit) fmtUsd(usd(sats)) else commas(sats)
}

private fun p2(n: Int) = n.toString().padStart(2, '0')
