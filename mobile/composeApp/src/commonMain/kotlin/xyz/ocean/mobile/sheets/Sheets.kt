package xyz.ocean.mobile.sheets

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.launch
import qrcode.QRCode
import xyz.ocean.mobile.data.Dir
import xyz.ocean.mobile.data.Rail
import xyz.ocean.mobile.data.Tx
import xyz.ocean.mobile.data.TxStatus
import xyz.ocean.mobile.data.WalletRepository
import xyz.ocean.mobile.data.amt
import xyz.ocean.mobile.data.commas
import xyz.ocean.mobile.data.fmtUsd
import xyz.ocean.mobile.data.short
import xyz.ocean.mobile.data.usd
import xyz.ocean.mobile.theme.OceanColors
import xyz.ocean.mobile.theme.OceanType
import xyz.ocean.mobile.ui.BtnVariant
import xyz.ocean.mobile.ui.MaturityCard
import xyz.ocean.mobile.ui.OButton
import xyz.ocean.mobile.ui.OIcon
import xyz.ocean.mobile.ui.Pill
import xyz.ocean.mobile.ui.PillTone

// ── shared sheet scaffold ──
@Composable
private fun Scrim(onClose: (() -> Unit)?) {
    Box(Modifier.fillMaxSize().background(Color.Black.copy(alpha = 0.6f)).then(if (onClose != null) Modifier.clickable { onClose() } else Modifier))
}

@Composable
private fun SheetShell(title: String, full: Boolean, onClose: (() -> Unit)?, onBack: (() -> Unit)? = null, right: (@Composable () -> Unit)? = null, footer: (@Composable RowScope.() -> Unit)? = null, body: @Composable () -> Unit) {
    Box(Modifier.fillMaxSize()) {
        Scrim(onClose)
        Column(
            Modifier.align(Alignment.BottomCenter).fillMaxWidth()
                .then(if (full) Modifier.fillMaxHeight() else Modifier)
                .clip(RoundedCornerShape(topStart = 24.dp, topEnd = 24.dp))
                .background(OceanColors.bgSecondary)
                .border(1.dp, OceanColors.borderStrong, RoundedCornerShape(topStart = 24.dp, topEnd = 24.dp)),
        ) {
            if (!full) Box(Modifier.padding(top = 10.dp, bottom = 4.dp).align(Alignment.CenterHorizontally).size(width = 38.dp, height = 5.dp).clip(RoundedCornerShape(999.dp)).background(OceanColors.borderStrong))
            Row(Modifier.fillMaxWidth().padding(start = 20.dp, end = 20.dp, top = if (full) 54.dp else 12.dp, bottom = 14.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                if (onBack != null) SheetIconBtn("chevL", onBack)
                Text(title, style = OceanType.sheetTitle.copy(color = OceanColors.fgPrimary), modifier = Modifier.weight(1f))
                if (right != null) right()
                if (onClose != null) SheetIconBtn("close", onClose)
            }
            Column(Modifier.weight(1f, fill = false).fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 20.dp)) { body() }
            if (footer != null) Row(Modifier.fillMaxWidth().border(0.dp, Color.Transparent).padding(start = 20.dp, end = 20.dp, top = 14.dp, bottom = 30.dp), horizontalArrangement = Arrangement.spacedBy(10.dp)) { footer() }
        }
    }
}

@Composable
private fun SheetIconBtn(icon: String, onClick: () -> Unit) {
    Box(Modifier.size(32.dp).clip(CircleShape).background(OceanColors.bgCard).border(1.dp, OceanColors.border, CircleShape).clickable { onClick() }, contentAlignment = Alignment.Center) {
        OIcon(icon, 16, OceanColors.fgSecondary)
    }
}

@Composable
fun CopyButton(value: String) {
    val clip = LocalClipboardManager.current
    var ok by remember { mutableStateOf(false) }
    Box(Modifier.size(30.dp).clip(RoundedCornerShape(8.dp)).background(OceanColors.bgSecondary).border(1.dp, OceanColors.border, RoundedCornerShape(8.dp)).clickable { clip.setText(AnnotatedString(value)); ok = true }, contentAlignment = Alignment.Center) {
        OIcon(if (ok) "check" else "copy", 15, OceanColors.fgSecondary)
    }
}

// ── Receive ──
@Composable
fun ReceiveSheet(repo: WalletRepository, onClose: () -> Unit) {
    var tab by remember { mutableStateOf("offer") }
    var invoice by remember { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(tab, repo) {
        if (tab == "invoice" && invoice == null) {
            error = null
            runCatching { repo.createInvoice(description = "OCEAN Lightning mobile") }
                .onSuccess { invoice = it }
                .onFailure { error = it.message ?: "Could not create an invoice." }
        }
    }
    val value = when (tab) {
        "offer" -> repo.offer
        "invoice" -> invoice
        else -> repo.miningAddress
    }
    val label = when (tab) { "offer" -> "Reusable BOLT12 offer"; "invoice" -> "Lightning invoice"; else -> "On-chain address" }
    SheetShell("Receive", full = false, onClose = onClose) {
        Row(Modifier.fillMaxWidth().padding(top = 6.dp).clip(RoundedCornerShape(999.dp)).background(OceanColors.bgSecondary).border(1.dp, OceanColors.border, RoundedCornerShape(999.dp)).padding(3.dp)) {
            listOf("offer" to "Offer", "invoice" to "Invoice", "onchain" to "On-chain").forEach { (k, l) ->
                val on = tab == k
                Box(Modifier.weight(1f).clip(RoundedCornerShape(999.dp)).background(if (on) OceanColors.bgCardHover else Color.Transparent).clickable { tab = k }.padding(vertical = 8.dp), contentAlignment = Alignment.Center) {
                    Text(l, style = OceanType.body.copy(fontSize = 13.sp, fontWeight = FontWeight.SemiBold, color = if (on) OceanColors.fgPrimary else OceanColors.fgTertiary))
                }
            }
        }
        Box(Modifier.fillMaxWidth().padding(vertical = 18.dp), contentAlignment = Alignment.Center) {
            Box(Modifier.size(190.dp).clip(RoundedCornerShape(8.dp)).background(Color(0xFFFAFAFA)), contentAlignment = Alignment.Center) {
                if (value != null) DestinationQr(value, Modifier.size(166.dp))
                else if (tab == "invoice" && error == null) {
                    androidx.compose.material3.CircularProgressIndicator(color = OceanColors.accent)
                } else {
                    OIcon("warn", 42, OceanColors.fgMuted)
                }
            }
        }
        Text(label.uppercase(), style = OceanType.sectionLabel)
        Spacer(Modifier.height(8.dp))
        Row(Modifier.fillMaxWidth().clip(RoundedCornerShape(8.dp)).background(OceanColors.bgCard).border(1.dp, OceanColors.border, RoundedCornerShape(8.dp)).padding(12.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Text(value?.let { short(it, 26, 14) } ?: error ?: "Unavailable", style = OceanType.monoSm.copy(fontSize = 11.5.sp), modifier = Modifier.weight(1f))
            if (value != null) CopyButton(value)
        }
        if (tab == "offer" && value != null) {
            Spacer(Modifier.height(8.dp))
            Text("This is the offer registered with OCEAN — payouts to it are flagged as verified.", style = OceanType.bodySm.copy(color = OceanColors.fgMuted, fontSize = 11.5.sp))
        }
        Spacer(Modifier.height(20.dp))
    }
}

@Composable
private fun DestinationQr(value: String, modifier: Modifier = Modifier) {
    val qrCode = remember(value) { QRCode.ofSquares().build(value) }
    Canvas(modifier.aspectRatio(1f)) {
        val modules = qrCode.rawData
        val quietZone = 4
        val dimension = modules.size + quietZone * 2
        val cell = size.minDimension / dimension
        modules.forEachIndexed { row, cells ->
            cells.forEachIndexed { col, square ->
                if (square.dark) {
                    drawRect(
                        color = Color.Black,
                        topLeft = Offset((col + quietZone) * cell, (row + quietZone) * cell),
                        size = Size(cell, cell),
                    )
                }
            }
        }
    }
}

// ── Transaction detail ──
@Composable
fun TxDetailSheet(t: Tx, usdUnit: Boolean, onClose: () -> Unit) {
    val inn = t.dir == Dir.IN
    val mat = t.status == TxStatus.MATURING
    SheetShell("Transaction", full = true, onClose = onClose) {
        // hero
        Column(Modifier.fillMaxWidth().padding(bottom = 20.dp), horizontalAlignment = Alignment.CenterHorizontally) {
            val dirColor = when { t.status == TxStatus.FAILED -> OceanColors.error; mat -> OceanColors.warning; inn -> OceanColors.success; else -> OceanColors.fgSecondary }
            Box(Modifier.size(52.dp).clip(CircleShape).background(OceanColors.bgCard).border(1.dp, OceanColors.border, CircleShape), contentAlignment = Alignment.Center) {
                OIcon(if (t.isOcean) "spark" else if (mat) "hourglass" else if (inn) "in" else "out", 26, if (t.isOcean) OceanColors.accent else dirColor)
            }
            Spacer(Modifier.height(14.dp))
            Row(verticalAlignment = Alignment.Bottom) {
                Text(amt(t.amt, usdUnit, t.dir), style = OceanType.detailAmount.copy(color = if (mat) OceanColors.warning else if (inn) OceanColors.success else OceanColors.fgPrimary))
                if (t.amt != null && !usdUnit) Text(" sats", style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 15.sp))
            }
            Spacer(Modifier.height(6.dp))
            Text(if (mat) "not yet spendable" else if (t.amt == null) "pending" else if (usdUnit) "${commas(t.amt)} sats" else fmtUsd(usd(t.amt)), style = OceanType.monoSm.copy(color = OceanColors.fgTertiary, fontSize = 13.sp))
            Spacer(Modifier.height(13.dp))
            StatusPill(t)
        }
        if (t.isOcean) {
            OceanBanner(t)
        } else if (t.dir == Dir.IN && t.offer && !t.noteMatch) {
            Caution(t)
        }
        if (mat && t.mat != null) {
            Spacer(Modifier.height(18.dp))
            MaturityCard(t, showNote = true)
        }
        SecLabel("Details")
        DetailRow(if (inn) "From" else "To", t.party)
        if (t.note != null) DetailRow("Note", t.note)
        DetailRow("Date", t.tsIso)
        DetailRow("Network", if (t.rail == Rail.LN) "Lightning" else "On-chain")
        if (t.fee != null) DetailRow("Fee", if (t.fee == 0L) "0 sats (free)" else "${commas(t.fee)} sats")
        SecLabel("Proof & references")
        if (t.rail == Rail.LN) {
            DetailRow("Payment hash", short(t.hash, 10, 8), copy = t.hash)
            if (t.preimage != null) DetailRow("Preimage", short(t.preimage, 10, 8), copy = t.preimage)
        } else {
            DetailRow("Transaction ID", short(t.txid, 10, 8), copy = t.txid)
            DetailRow("Confirmations", if ((t.conf ?: 0) >= 6) "${t.conf} (final)" else "${t.conf} / 6")
        }
        Spacer(Modifier.height(20.dp))
    }
}

@Composable
private fun StatusPill(t: Tx) = when (t.status) {
    TxStatus.SETTLED -> Pill(PillTone.OK, "Settled", "check")
    TxStatus.MATURING -> Pill(PillTone.MAT, "Maturing · ${t.mat?.confs}/${t.mat?.target}", "hourglass")
    TxStatus.PENDING -> Pill(PillTone.ACC, "In-flight")
    TxStatus.CONFIRMING -> Pill(PillTone.ACC, "${t.conf}/6 conf", "refresh")
    TxStatus.FAILED -> Pill(PillTone.ERR, "Failed", "warn")
}

@Composable
private fun OceanBanner(t: Tx) {
    Column(Modifier.fillMaxWidth().padding(vertical = 4.dp).clip(RoundedCornerShape(8.dp)).background(OceanColors.accentDim).border(1.dp, OceanColors.accentGlow, RoundedCornerShape(8.dp)).padding(14.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(9.dp)) {
            OIcon("spark", 15, OceanColors.accent)
            Text("Verified OCEAN payout", style = OceanType.body.copy(color = OceanColors.accent, fontWeight = FontWeight.SemiBold, fontSize = 13.sp))
        }
        Spacer(Modifier.height(10.dp))
        BannerLine("Paid to your registered OCEAN offer (BOLT12).")
        BannerLine("Payer note matches OCEAN's signature — ${t.payerNote}")
    }
}

@Composable
private fun BannerLine(text: String) {
    Row(Modifier.padding(top = 8.dp), horizontalArrangement = Arrangement.spacedBy(9.dp)) {
        OIcon("check", 15, OceanColors.success)
        Text(text, style = OceanType.bodySm.copy(color = OceanColors.fgSecondary, fontSize = 12.5.sp))
    }
}

@Composable
private fun Caution(t: Tx) {
    Row(Modifier.fillMaxWidth().padding(vertical = 4.dp).clip(RoundedCornerShape(8.dp)).background(OceanColors.warningDim).border(1.dp, OceanColors.warningLine, RoundedCornerShape(8.dp)).padding(13.dp), horizontalArrangement = Arrangement.spacedBy(9.dp)) {
        OIcon("warn", 16, OceanColors.warning)
        Text("Paid to your offer, but the payer note ${t.payerNote ?: "—"} doesn't match OCEAN's signature. Treated as a generic payment.", style = OceanType.bodySm.copy(color = OceanColors.fgSecondary, fontSize = 12.5.sp))
    }
}

@Composable
private fun SecLabel(text: String) {
    Text(text.uppercase(), style = OceanType.monoXs.copy(color = OceanColors.fgMuted, letterSpacing = 0.7.sp), modifier = Modifier.padding(top = 20.dp, bottom = 4.dp))
}

@Composable
private fun DetailRow(k: String, v: String, copy: String? = null) {
    Row(Modifier.fillMaxWidth().padding(vertical = 11.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
        Text(k, style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 12.5.sp))
        Spacer(Modifier.weight(1f))
        Text(v, style = OceanType.monoSm.copy(fontSize = 12.5.sp))
        if (copy != null) CopyButton(copy)
    }
}

// ── Send (dest → amount → review → sending → done) ──
private data class Dest(val key: String, val name: String, val icon: String, val onchain: Boolean)
private val DESTS = listOf(
    Dest("invoice", "Invoice", "bolt", false),
    Dest("lnaddr", "LN address", "at", false),
    Dest("onchain", "On-chain", "btc", true),
    Dest("exchange", "Exchange", "bank", true),
)

@Composable
fun SendSheet(repo: WalletRepository, usdUnit: Boolean, onClose: () -> Unit) {
    var step by remember { mutableStateOf(1) } // 1 dest 2 amount 3 review 98 sending 99 done
    var destKey by remember { mutableStateOf("invoice") }
    var value by remember { mutableStateOf("") }
    var amount by remember { mutableStateOf(0L) }
    var proof by remember { mutableStateOf("") }
    var sendError by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val dest = DESTS.first { it.key == destKey }
    val bal by produceState<xyz.ocean.mobile.data.Balances?>(null, repo) {
        this.value = runCatching { repo.balances() }.getOrNull()
    }
    val available = bal ?: xyz.ocean.mobile.data.Balances(0, 1, 0)
    val srcBal = if (dest.onchain) available.onchain else available.channel
    val srcName = if (dest.onchain) "On-chain balance" else "Lightning channel"
    val fee = if (dest.onchain) 2400L else maxOf(1L, (amount * 0.0006).toLong())

    when (step) {
        98 -> SheetShell("Sending", full = true, onClose = null) {
            Column(Modifier.fillMaxWidth().padding(vertical = 24.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                Box(Modifier.size(64.dp).clip(CircleShape).background(OceanColors.accentDim).border(1.dp, OceanColors.accentGlow, CircleShape))
                Spacer(Modifier.height(18.dp))
                Text("Routing payment…", style = OceanType.sheetTitle.copy(fontSize = 21.sp))
                Text(if (dest.onchain) "Broadcasting transaction" else "Finding a path through the network", style = OceanType.monoSm.copy(color = OceanColors.fgTertiary))
            }
        }
        99 -> SheetShell("Sent", full = true, onClose = onClose, footer = { Box(Modifier.weight(1f)) { OButton("Done", fill = true, onClick = onClose) } }) {
            Column(Modifier.fillMaxWidth().padding(vertical = 24.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                Box(Modifier.size(64.dp).clip(CircleShape).background(OceanColors.successDim).border(1.dp, OceanColors.success.copy(alpha = 0.4f), CircleShape), contentAlignment = Alignment.Center) {
                    OIcon("check", 32, OceanColors.success)
                }
                Spacer(Modifier.height(18.dp))
                Text("Payment sent", style = OceanType.sheetTitle.copy(fontSize = 21.sp))
                Text("${commas(amount)} sats · ${fmtUsd(usd(amount))}", style = OceanType.monoSm.copy(color = OceanColors.fgTertiary))
                Spacer(Modifier.height(20.dp))
                Column(Modifier.fillMaxWidth().clip(RoundedCornerShape(8.dp)).background(OceanColors.bgCard).border(1.dp, OceanColors.border, RoundedCornerShape(8.dp)).padding(13.dp)) {
                    Text((if (dest.onchain) "TRANSACTION ID" else "PAYMENT REFERENCE"), style = OceanType.monoXs.copy(letterSpacing = 0.6.sp))
                    Spacer(Modifier.height(7.dp))
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(9.dp)) {
                        Text(short(proof, 14, 10), style = OceanType.monoSm.copy(fontSize = 11.5.sp), modifier = Modifier.weight(1f))
                        CopyButton(proof)
                    }
                }
            }
        }
        1 -> SheetShell("Send bitcoin", full = false, onClose = onClose, right = { Dots(1) }, footer = {
            Box(Modifier.weight(1f)) { OButton("Continue", iconRight = "chevR", enabled = value.trim().length > 4, fill = true) { step = 2 } }
        }) {
            FieldLabel("Destination")
            DestGrid(destKey) { destKey = it }
            when (destKey) {
                "invoice" -> { FieldLabel("BOLT11 invoice"); InputField(value, "lnbc1…  Paste or scan", multiline = true) { value = it } }
                "lnaddr" -> { FieldLabel("Lightning address"); InputField(value, "you@domain.com") { value = it } }
                "onchain" -> { FieldLabel("Bitcoin address"); InputField(value, "bc1q…") { value = it } }
                "exchange" -> { FieldLabel("Choose a service"); Text("Kraken · Coinbase · River · Strike", style = OceanType.bodySm.copy(color = OceanColors.fgTertiary)) }
            }
            Spacer(Modifier.height(12.dp))
        }
        2 -> SheetShell("Amount", full = false, onClose = onClose, onBack = { step = 1 }, right = { Dots(2) }, footer = {
            OButton("Back", BtnVariant.GHOST) { step = 1 }
            Box(Modifier.weight(1f)) { OButton("Review", enabled = amount in 1..srcBal, fill = true) { step = 3 } }
        }) {
            Column(Modifier.fillMaxWidth().padding(vertical = 18.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                BasicTextField(
                    value = if (amount > 0) commas(amount) else "",
                    onValueChange = { s -> amount = s.filter { it.isDigit() }.toLongOrNull() ?: 0L },
                    textStyle = OceanType.heroValue.copy(color = OceanColors.fgPrimary, fontSize = 46.sp),
                    keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(keyboardType = KeyboardType.Number),
                    cursorBrush = androidx.compose.ui.graphics.SolidColor(OceanColors.accent),
                )
                Text(if (amount > 0) (if (usdUnit) "${commas(amount)} sats" else "≈ ${fmtUsd(usd(amount))}") else "≈ \$0.00", style = OceanType.monoSm.copy(color = OceanColors.fgTertiary))
            }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp, Alignment.CenterHorizontally)) {
                ToolBtn("Max") { amount = maxOf(0, srcBal - fee) }
                ToolBtn("50%") { amount = srcBal / 2 }
            }
            Spacer(Modifier.height(16.dp))
            Row(Modifier.fillMaxWidth().clip(RoundedCornerShape(8.dp)).background(OceanColors.bgCard).border(1.dp, OceanColors.border, RoundedCornerShape(8.dp)).padding(12.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(9.dp)) {
                OIcon(if (dest.onchain) "btc" else "bolt", 17, OceanColors.accent)
                Text("From $srcName · ${commas(srcBal)} sats available", style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 12.5.sp))
            }
        }
        else -> { // 3: review
            val total = amount + fee
            SheetShell("Review & confirm", full = false, onClose = onClose, onBack = { step = 2 }, right = { Dots(3) }, footer = {
                OButton("Back", BtnVariant.GHOST) { step = 2 }
                Box(Modifier.weight(1f)) {
                    OButton("Confirm", icon = "bolt", fill = true) {
                        step = 98
                        sendError = null
                        scope.launch {
                            runCatching { repo.pay(value.trim(), amount, null) }
                                .onSuccess {
                                    proof = it.id
                                    step = 99
                                }
                                .onFailure {
                                    sendError = it.message ?: "Payment failed."
                                    step = 3
                                }
                        }
                    }
                }
            }) {
                Spacer(Modifier.height(6.dp))
                ReviewRow("To", if (value.length > 26) short(value, 10, 8) else value.ifEmpty { dest.name })
                ReviewRow("Network", if (dest.onchain) "On-chain" else "Lightning")
                ReviewRow("Amount", "${commas(amount)} sats")
                ReviewRow("Network fee", "${commas(fee)} sats")
                ReviewRow("From", srcName)
                Row(Modifier.fillMaxWidth().padding(top = 15.dp).border(0.dp, Color.Transparent), verticalAlignment = Alignment.Bottom) {
                    Text("Total", style = OceanType.body.copy(fontWeight = FontWeight.SemiBold, fontSize = 14.sp))
                    Spacer(Modifier.weight(1f))
                    Text("${commas(total)}", style = OceanType.splitValue.copy(color = OceanColors.fgPrimary, fontSize = 20.sp))
                    Text(" sats", style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 12.sp))
                }
                Spacer(Modifier.height(14.dp))
                Text("≈ ${fmtUsd(usd(total))} · Bitcoin payments can't be reversed — check the destination.", style = OceanType.bodySm.copy(color = OceanColors.fgMuted, fontSize = 11.5.sp), modifier = Modifier.fillMaxWidth())
                if (sendError != null) {
                    Spacer(Modifier.height(10.dp))
                    Text(sendError!!, style = OceanType.bodySm.copy(color = OceanColors.error))
                }
                Spacer(Modifier.height(12.dp))
            }
        }
    }
}

@Composable private fun Dots(n: Int) = Row(horizontalArrangement = Arrangement.spacedBy(5.dp)) {
    (1..3).forEach { Box(Modifier.size(6.dp).clip(CircleShape).background(if (it <= n) OceanColors.accent else OceanColors.borderStrong)) }
}
@Composable private fun FieldLabel(text: String) = Text(text.uppercase(), style = OceanType.sectionLabel, modifier = Modifier.padding(top = 16.dp, bottom = 8.dp))
@Composable private fun ToolBtn(text: String, onClick: () -> Unit) = Box(Modifier.clip(RoundedCornerShape(999.dp)).background(OceanColors.bgCard).border(1.dp, OceanColors.borderStrong, RoundedCornerShape(999.dp)).clickable { onClick() }.padding(horizontal = 14.dp, vertical = 7.dp)) { Text(text, style = OceanType.monoSm.copy(fontSize = 12.sp)) }
@Composable private fun ReviewRow(k: String, v: String) = Row(Modifier.fillMaxWidth().padding(vertical = 13.dp), verticalAlignment = Alignment.CenterVertically) {
    Text(k, style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, fontSize = 13.sp)); Spacer(Modifier.weight(1f)); Text(v, style = OceanType.monoSm.copy(fontSize = 13.sp))
}

@Composable
private fun InputField(value: String, placeholder: String, multiline: Boolean = false, onChange: (String) -> Unit) {
    Box(Modifier.fillMaxWidth().clip(RoundedCornerShape(8.dp)).background(OceanColors.bgCard).border(1.dp, OceanColors.borderStrong, RoundedCornerShape(8.dp)).padding(13.dp).then(if (multiline) Modifier.height(86.dp) else Modifier)) {
        if (value.isEmpty()) Text(placeholder, style = OceanType.monoSm.copy(color = OceanColors.fgMuted, fontSize = 14.sp))
        BasicTextField(value = value, onValueChange = onChange, textStyle = OceanType.monoSm.copy(color = OceanColors.fgPrimary, fontSize = 14.sp), cursorBrush = androidx.compose.ui.graphics.SolidColor(OceanColors.accent), modifier = Modifier.fillMaxWidth())
    }
}

@Composable
private fun DestGrid(selected: String, onSelect: (String) -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(9.dp)) {
        DESTS.chunked(2).forEach { rowItems ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(9.dp)) {
                rowItems.forEach { d ->
                    val on = d.key == selected
                    Row(Modifier.weight(1f).clip(RoundedCornerShape(8.dp)).background(if (on) OceanColors.accentDim else OceanColors.bgCard).border(1.dp, if (on) OceanColors.accent else OceanColors.border, RoundedCornerShape(8.dp)).clickable { onSelect(d.key) }.padding(13.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        OIcon(d.icon, 17, if (on) OceanColors.accent else OceanColors.fgSecondary)
                        Text(d.name, style = OceanType.bodySm.copy(color = if (on) OceanColors.accent else OceanColors.fgSecondary, fontSize = 13.sp))
                    }
                }
            }
        }
    }
}
