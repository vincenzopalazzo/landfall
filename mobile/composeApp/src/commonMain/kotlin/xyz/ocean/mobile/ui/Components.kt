package xyz.ocean.mobile.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import xyz.ocean.mobile.data.Dir
import xyz.ocean.mobile.data.Tx
import xyz.ocean.mobile.data.TxStatus
import xyz.ocean.mobile.data.amt
import xyz.ocean.mobile.data.commas
import xyz.ocean.mobile.data.fmtUsd
import xyz.ocean.mobile.data.matEta
import xyz.ocean.mobile.data.rel
import xyz.ocean.mobile.data.usd
import xyz.ocean.mobile.theme.OceanColors
import xyz.ocean.mobile.theme.OceanType
import xyz.ocean.mobile.theme.oceanIcon

// ── small primitives ──
@Composable
fun OIcon(name: String, size: Int = 20, tint: Color = OceanColors.fgSecondary, modifier: Modifier = Modifier) {
    Icon(oceanIcon(name), contentDescription = null, tint = tint, modifier = modifier.size(size.dp))
}

@Composable
fun StatusDot(color: Color = OceanColors.success, size: Int = 7) {
    Box(Modifier.size(size.dp).clip(CircleShape).background(color))
}

@Composable
fun OCard(modifier: Modifier = Modifier, padding: Int = 16, content: @Composable () -> Unit) {
    Column(
        modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(12.dp))
            .background(OceanColors.bgCard)
            .border(1.dp, OceanColors.borderSubtle, RoundedCornerShape(12.dp))
            .padding(padding.dp)
    ) { content() }
}

enum class PillTone { OK, MAT, ACC, ERR, MUTED }

@Composable
fun Pill(tone: PillTone, text: String, leading: String? = null) {
    val (fg, bg) = when (tone) {
        PillTone.OK -> OceanColors.success to OceanColors.successDim
        PillTone.MAT -> OceanColors.warning to OceanColors.warningDim
        PillTone.ACC -> OceanColors.accent to OceanColors.accentDim
        PillTone.ERR -> OceanColors.error to OceanColors.errorDim
        PillTone.MUTED -> OceanColors.fgTertiary to OceanColors.bgSecondary
    }
    Row(
        Modifier.clip(RoundedCornerShape(999.dp)).background(bg).padding(horizontal = 9.dp, vertical = 3.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(5.dp),
    ) {
        if (leading != null) OIcon(leading, 11, fg)
        Text(text.uppercase(), style = OceanType.pill.copy(color = fg))
    }
}

@Composable
fun SectionLabel(text: String, more: String? = null, onMore: (() -> Unit)? = null) {
    Row(
        Modifier.fillMaxWidth().padding(horizontal = 4.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(text.uppercase(), style = OceanType.sectionLabel)
        Spacer(Modifier.weight(1f))
        if (more != null && onMore != null) {
            Row(
                Modifier.clickable { onMore() },
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(2.dp),
            ) {
                Text(more, fontSize = 12.sp, color = OceanColors.fgTertiary)
                OIcon("chevR", 13, OceanColors.fgTertiary)
            }
        }
    }
}

enum class BtnVariant { PRIMARY, GHOST, SUBTLE }

@Composable
fun OButton(
    text: String,
    variant: BtnVariant = BtnVariant.PRIMARY,
    icon: String? = null,
    iconRight: String? = null,
    enabled: Boolean = true,
    fill: Boolean = false,
    onClick: () -> Unit,
) {
    val (bg, fg, borderC) = when (variant) {
        BtnVariant.PRIMARY -> Triple(OceanColors.accent, OceanColors.onAccent, Color.Transparent)
        BtnVariant.GHOST -> Triple(OceanColors.bgSecondary, OceanColors.fgPrimary, OceanColors.borderStrong)
        BtnVariant.SUBTLE -> Triple(OceanColors.bgCard, OceanColors.fgSecondary, OceanColors.border)
    }
    val alpha = if (enabled) 1f else 0.4f
    Row(
        (if (fill) Modifier.fillMaxWidth() else Modifier)
            .clip(RoundedCornerShape(999.dp))
            .background(bg.copy(alpha = bg.alpha * alpha))
            .border(1.dp, borderC, RoundedCornerShape(999.dp))
            .clickable(enabled = enabled) { onClick() }
            .padding(horizontal = 18.dp, vertical = 12.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (icon != null) OIcon(icon, 17, fg.copy(alpha = alpha))
        Text(text, style = OceanType.button.copy(color = fg.copy(alpha = alpha)))
        if (iconRight != null) OIcon(iconRight, 17, fg.copy(alpha = alpha))
    }
}

// ── activity row (m-txrow) ──
@Composable
fun TxRow(t: Tx, usdUnit: Boolean, nowMs: Long, compact: Boolean = false, onOpen: (Tx) -> Unit) {
    val oc = t.isOcean
    val inn = t.dir == Dir.IN
    val mat = t.status == TxStatus.MATURING
    val icTint = when {
        t.status == TxStatus.FAILED -> OceanColors.error
        mat -> OceanColors.warning
        inn -> OceanColors.success
        else -> OceanColors.fgSecondary
    }
    val amtColor = when {
        mat -> OceanColors.warning
        inn && t.amt != null -> OceanColors.success
        else -> OceanColors.fgPrimary
    }
    Row(
        Modifier
            .fillMaxWidth()
            .clickable { onOpen(t) }
            .then(if (oc) Modifier.background(OceanColors.accentDim, RoundedCornerShape(8.dp)) else Modifier)
            .padding(horizontal = 8.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(13.dp),
    ) {
        // icon bubble
        Box(
            Modifier.size(36.dp).clip(CircleShape)
                .background(if (oc) OceanColors.accent else OceanColors.bgSecondary)
                .border(1.dp, if (oc) OceanColors.accent else OceanColors.border, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            OIcon(
                if (oc) "spark" else if (mat) "hourglass" else if (inn) "in" else "out",
                18, if (oc) OceanColors.onAccent else icTint,
            )
        }
        Column(Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(7.dp)) {
                Text(t.party, style = OceanType.body.copy(fontWeight = FontWeight.SemiBold), maxLines = 1, overflow = TextOverflow.Ellipsis)
                if (oc) Pill(PillTone.ACC, "Payout")
            }
            if (!compact) {
                val sub = if (mat) "Matures ${matEta(t.mat!!)}"
                else (if (t.rail == xyz.ocean.mobile.data.Rail.LN) "Lightning" else "On-chain") + " · " + rel(t.tsMs, nowMs)
                Text(sub, style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 11.5.sp))
            }
        }
        Column(horizontalAlignment = Alignment.End) {
            Text(
                amt(t.amt, usdUnit, t.dir) + (if (t.amt != null && !usdUnit) " sats" else ""),
                style = OceanType.monoSm.copy(color = amtColor, fontWeight = FontWeight.Medium, fontSize = 14.sp),
            )
            val f = when {
                mat -> "not spendable"
                t.amt == null -> "pending"
                usdUnit -> commas(t.amt) + " sats"
                else -> fmtUsd(usd(t.amt))
            }
            Text(f, style = OceanType.monoXs)
        }
    }
}

// ── maturity card (m-mat) ──
@Composable
fun MaturityCard(t: Tx, showNote: Boolean = true, onSimulate: (() -> Unit)? = null) {
    val m = t.mat ?: return
    Column(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp))
            .background(OceanColors.warningDim)
            .border(1.dp, OceanColors.warningLine, RoundedCornerShape(12.dp))
            .padding(16.dp),
    ) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OIcon("hourglass", 15, OceanColors.warning)
                Text("Maturing", style = OceanType.body.copy(color = OceanColors.warning, fontWeight = FontWeight.SemiBold, fontSize = 13.sp))
            }
            Spacer(Modifier.weight(1f))
            Text(commas(t.amt ?: 0) + " sats", style = OceanType.monoSm.copy(color = OceanColors.fgPrimary, fontSize = 16.sp))
        }
        Spacer(Modifier.height(11.dp))
        ProgressBar(fraction = xyz.ocean.mobile.data.matPct(m) / 100f, color = OceanColors.warning)
        Spacer(Modifier.height(8.dp))
        Row(Modifier.fillMaxWidth()) {
            Text("${m.confs} / ${m.target} confirmations", style = OceanType.monoXs)
            Spacer(Modifier.weight(1f))
            Text("${matEta(m)} left", style = OceanType.monoXs)
        }
        if (showNote) {
            Spacer(Modifier.height(12.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OIcon("info", 14, OceanColors.warning)
                Text(
                    "Mined rewards can't be spent until ${m.target} confirmations. Each new block OCEAN finds resets the window — anchored to block ${commas(m.block)}.",
                    style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 11.5.sp),
                )
            }
        }
        if (onSimulate != null) {
            Spacer(Modifier.height(12.dp))
            Row(
                Modifier.fillMaxWidth().clip(RoundedCornerShape(8.dp))
                    .background(OceanColors.warningDim)
                    .clickable { onSimulate() }
                    .padding(11.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OIcon("cube", 14, OceanColors.warning)
                Text("Simulate: OCEAN finds a new block", style = OceanType.bodySm.copy(color = OceanColors.warning, fontWeight = FontWeight.SemiBold, fontSize = 12.5.sp))
            }
        }
    }
}

@Composable
fun ProgressBar(fraction: Float, color: Color = OceanColors.accent) {
    Box(
        Modifier.fillMaxWidth().height(6.dp).clip(RoundedCornerShape(999.dp))
            .background(OceanColors.bgSecondary).border(1.dp, OceanColors.border, RoundedCornerShape(999.dp)),
    ) {
        Box(Modifier.fillMaxWidth(fraction.coerceIn(0f, 1f)).height(6.dp).clip(RoundedCornerShape(999.dp)).background(color))
    }
}

@Composable
fun StatCard(icon: String, label: String, value: String, valueUnit: String? = null, sub: String, valueColor: Color = OceanColors.fgPrimary, modifier: Modifier = Modifier) {
    Column(
        modifier.clip(RoundedCornerShape(8.dp)).background(OceanColors.bgCard)
            .border(1.dp, OceanColors.borderSubtle, RoundedCornerShape(8.dp)).padding(14.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            OIcon(icon, 13, OceanColors.fgMuted)
            Text(label, style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 11.sp))
        }
        Spacer(Modifier.height(9.dp))
        Row(verticalAlignment = Alignment.Bottom) {
            Text(value, style = OceanType.statValue.copy(color = valueColor))
            if (valueUnit != null) Text(" $valueUnit", style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 12.sp))
        }
        Spacer(Modifier.height(5.dp))
        Text(sub, style = OceanType.monoXs)
    }
}
