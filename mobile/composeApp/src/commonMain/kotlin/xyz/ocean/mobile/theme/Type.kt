package xyz.ocean.mobile.theme

import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

// Type roles map to the design's two families:
//   --font-display / --font-sans → Inter   (here: FontFamily.Default placeholder)
//   --font-mono                  → Geist Mono (here: FontFamily.Monospace placeholder)
// The real Inter + Geist Mono files are vendored under composeApp resources and
// wired here — see mobile/README.md → "Fonts". Sizes/weights below are 1-1 with
// mobile.css so swapping the family in is the only change needed.
object OceanType {
    val display = FontFamily.Default
    val sans = FontFamily.Default
    val mono = FontFamily.Monospace

    // header title (.m-head-t: 21/700)
    val headTitle = TextStyle(fontFamily = display, fontSize = 21.sp, fontWeight = FontWeight.Bold, letterSpacing = (-0.5).sp)
    val headSub = TextStyle(fontFamily = sans, fontSize = 11.sp, color = OceanColors.fgTertiary)

    // section label (.m-sec-lbl: 11/600 uppercase)
    val sectionLabel = TextStyle(fontFamily = sans, fontSize = 11.sp, fontWeight = FontWeight.SemiBold, letterSpacing = 0.8.sp, color = OceanColors.fgMuted)

    // hero balance (.m-hero-v: mono 40/500)
    val heroValue = TextStyle(fontFamily = mono, fontSize = 40.sp, fontWeight = FontWeight.Medium, letterSpacing = (-1.2).sp)
    // hashrate value (.m-hr-v: mono 38/500)
    val hashrateValue = TextStyle(fontFamily = mono, fontSize = 38.sp, fontWeight = FontWeight.Medium, letterSpacing = (-1.1).sp)
    // stat value (.m-stat .v: mono 22/500)
    val statValue = TextStyle(fontFamily = mono, fontSize = 22.sp, fontWeight = FontWeight.Medium, letterSpacing = (-0.4).sp)
    // split value (.m-split-v: mono 19/500)
    val splitValue = TextStyle(fontFamily = mono, fontSize = 19.sp, fontWeight = FontWeight.Medium)
    // detail amount (.m-dt-amt: mono 34/500)
    val detailAmount = TextStyle(fontFamily = mono, fontSize = 34.sp, fontWeight = FontWeight.Medium, letterSpacing = (-0.7).sp)

    val sheetTitle = TextStyle(fontFamily = display, fontSize = 18.sp, fontWeight = FontWeight.Bold, letterSpacing = (-0.3).sp)
    val body = TextStyle(fontFamily = sans, fontSize = 14.sp, color = OceanColors.fgPrimary)
    val bodySm = TextStyle(fontFamily = sans, fontSize = 12.sp, color = OceanColors.fgSecondary)
    val monoSm = TextStyle(fontFamily = mono, fontSize = 12.sp, color = OceanColors.fgSecondary)
    val monoXs = TextStyle(fontFamily = mono, fontSize = 10.5.sp, color = OceanColors.fgMuted)

    // button label (.m-btn: 15/600)
    val button = TextStyle(fontFamily = sans, fontSize = 15.sp, fontWeight = FontWeight.SemiBold)
    // pill (.m-pill: mono 10 uppercase)
    val pill = TextStyle(fontFamily = mono, fontSize = 10.sp, fontWeight = FontWeight.Medium, letterSpacing = 0.4.sp)
}
