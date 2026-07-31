package xyz.ocean.mobile.data

import androidx.compose.runtime.Composable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.compositionLocalOf
import kotlin.math.abs
import kotlin.math.roundToLong

/**
 * Live BTC/USD spot, supplied by `App` from `OceanlnCore.btcUsd()`.
 *
 * `0.0` means "no rate available" — offline, blocked, or a bad response. It is
 * the default here precisely so a screen that forgets to provide it shows sats
 * rather than a made-up dollar figure. Same contract as the web's `price.ts`.
 */
val LocalBtcUsd = compositionLocalOf { 0.0 }

/** Whether fiat can be displayed at all. Drives the sats/USD toggle. */
@Composable
@ReadOnlyComposable
fun fiatAvailable(): Boolean = LocalBtcUsd.current > 0.0

fun commas(n: Long): String {
    val s = abs(n).toString()
    val sb = StringBuilder()
    for ((i, c) in s.withIndex()) {
        if (i > 0 && (s.length - i) % 3 == 0) sb.append(',')
        sb.append(c)
    }
    return (if (n < 0) "-" else "") + sb.toString()
}

/**
 * Sats → USD at the live rate, or `null` when there is no rate. Callers must
 * handle `null` by showing sats; there is deliberately no fallback constant.
 */
@Composable
@ReadOnlyComposable
fun usdOrNull(sats: Long): Double? {
    val rate = LocalBtcUsd.current
    return if (rate > 0.0) (sats / 1e8) * rate else null
}

fun fmtUsd(v: Double): String {
    val cents = (v * 100).roundToLong()
    val whole = cents / 100
    val frac = (abs(cents) % 100).toString().padStart(2, '0')
    return "$" + commas(whole) + "." + frac
}

/** Formatted fiat for `sats`, or `null` when no rate is available. */
@Composable
@ReadOnlyComposable
fun fiatOrNull(sats: Long): String? = usdOrNull(sats)?.let { fmtUsd(it) }

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

/**
 * Hashes/sec → a human unit. OCEAN reports raw H/s; miners read Th/s or Ph/s,
 * so pick the unit from the magnitude instead of assuming one.
 */
fun hashrate(hps: Double): Pair<String, String> {
    if (!hps.isFinite() || hps <= 0.0) return "0" to "H/s"
    val units = listOf("H/s", "Kh/s", "Mh/s", "Gh/s", "Th/s", "Ph/s", "Eh/s")
    var v = hps
    var i = 0
    while (v >= 1000.0 && i < units.lastIndex) {
        v /= 1000.0
        i++
    }
    val text = if (v >= 100) v.roundToLong().toString() else ((v * 10).roundToLong() / 10.0).toString()
    return text to units[i]
}

/** Compact percentage, e.g. `0.033` → "0.033". Avoids float noise in the UI. */
fun pct(v: Double): String {
    if (!v.isFinite() || v <= 0.0) return "0"
    val scaled = if (v < 1.0) (v * 1000).roundToLong() / 1000.0 else (v * 100).roundToLong() / 100.0
    return scaled.toString()
}

/**
 * Epoch millis → `YYYY-MM-DD HH:MM` UTC, for the transaction detail sheet.
 *
 * Hand-rolled because the project has no kotlinx-datetime dependency and this
 * has to work identically on JVM and Kotlin/Native. Days→civil date is Howard
 * Hinnant's `civil_from_days`, valid across the whole Unix range.
 */
fun isoUtc(tsMs: Long): String {
    if (tsMs <= 0L) return "—"
    val totalSeconds = floorDiv(tsMs, 1000L)
    val days = floorDiv(totalSeconds, 86_400L)
    val secondOfDay = totalSeconds - days * 86_400L

    // civil_from_days: shift the epoch to 0000-03-01 so leap days land last.
    val z = days + 719_468L
    val era = floorDiv(z, 146_097L)
    val doe = z - era * 146_097L                                   // [0, 146096]
    val yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365
    val y = yoe + era * 400L
    val doy = doe - (365 * yoe + yoe / 4 - yoe / 100)              // [0, 365]
    val mp = (5 * doy + 2) / 153                                   // [0, 11]
    val d = doy - (153 * mp + 2) / 5 + 1                           // [1, 31]
    val m = if (mp < 10) mp + 3 else mp - 9                        // [1, 12]
    val year = if (m <= 2) y + 1 else y

    val hh = secondOfDay / 3600
    val mm = (secondOfDay % 3600) / 60
    return "$year-${p2(m)}-${p2(d)} ${p2(hh)}:${p2(mm)}"
}

private fun p2(n: Long): String = if (n < 10) "0$n" else n.toString()

/** Floor division. `kotlin.Long./` truncates toward zero, which breaks the
 *  civil-date maths for pre-1970 timestamps; `java.lang.Math` is not available
 *  in commonMain, so this is the portable equivalent. */
private fun floorDiv(a: Long, b: Long): Long {
    val q = a / b
    return if (a % b != 0L && (a xor b) < 0L) q - 1 else q
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

/**
 * Signed amount honouring the unit toggle. Falls back to sats when the toggle
 * says USD but no rate is available, so a missing price can never render as a
 * wrong price.
 */
@Composable
@ReadOnlyComposable
fun amt(sats: Long?, usdUnit: Boolean, dir: Dir?): String {
    if (sats == null) return "—"
    val sign = when (dir) { Dir.IN -> "+"; Dir.OUT -> "−"; else -> "" }
    val fiat = if (usdUnit) fiatOrNull(sats) else null
    return sign + (fiat ?: commas(sats))
}
