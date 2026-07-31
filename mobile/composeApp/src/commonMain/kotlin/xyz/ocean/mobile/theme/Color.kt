package xyz.ocean.mobile.theme

import androidx.compose.ui.graphics.Color

// OCEAN dashboard palette — 1-1 with the `[data-theme="dashboard"]` block in
// colors_and_type.css (dark surface + Bitcoin-orange accent).
object OceanColors {
    val bgPrimary = Color(0xFF09090B)
    val bgSecondary = Color(0xFF0F0F11)
    val bgTertiary = Color(0xFF131316)
    val bgCard = Color(0xFF131316)
    val bgCardHover = Color(0xFF18181C)

    val fgPrimary = Color(0xFFFAFAFA)
    val fgSecondary = Color(0xFFA1A1AA)
    val fgTertiary = Color(0xFF71717A)
    val fgMuted = Color(0xFF63636E)

    val border = Color(0xFF1E1E23)
    val borderStrong = Color(0xFF2A2A30)
    val borderSubtle = Color(0xFF16161A)

    // Bitcoin orange.
    val accent = Color(0xFFF7931A)
    val accentHover = Color(0xFFFFA733)
    val accentDim = Color(0x14F7931A)   // rgba(247,147,26,0.08) ≈ 0x14 alpha
    val accentGlow = Color(0x26F7931A)  // rgba(247,147,26,0.15) ≈ 0x26 alpha
    val onAccent = Color(0xFF0B0B0D)

    val success = Color(0xFF22C55E)
    val successDim = Color(0x1A22C55E)
    val warning = Color(0xFFFFB300)
    val warningDim = Color(0x0DFFB300)  // rgba(255,179,0,0.05)
    val warningLine = Color(0x38FFB300) // rgba(255,179,0,0.22)
    val error = Color(0xFFEF4444)
    val errorDim = Color(0x1AEF4444)

    // On-chain accent (blue) used on the Lightning/on-chain split.
    val onchain = Color(0xFF3EA0FF)
    val onchainDim = Color(0x1F1489EE)
}
