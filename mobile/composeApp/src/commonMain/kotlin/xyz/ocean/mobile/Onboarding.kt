package xyz.ocean.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.launch
import xyz.ocean.mobile.data.WalletRepository
import xyz.ocean.mobile.theme.OceanColors
import xyz.ocean.mobile.theme.OceanType
import xyz.ocean.mobile.ui.BtnVariant
import xyz.ocean.mobile.ui.OButton
import xyz.ocean.mobile.ui.OIcon

private enum class OnboardingStep { WELCOME, CREATING, PHRASE, CONFIRM, RESTORE }

private val quizPositions = listOf(3, 12, 20)
private val decoys = listOf("anchor", "tide", "forest", "puzzle", "candle", "quartz")

@Composable
fun Onboarding(repo: WalletRepository, onComplete: () -> Unit) {
    var step by remember { mutableStateOf(OnboardingStep.WELCOME) }
    var mnemonic by remember { mutableStateOf<List<String>>(emptyList()) }
    var revealed by remember { mutableStateOf(false) }
    var backedUp by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val restoreWords = remember { mutableStateListOf(*Array(24) { "" }) }
    val answers = remember { mutableStateListOf("", "", "") }
    val scope = rememberCoroutineScope()

    fun create() {
        step = OnboardingStep.CREATING
        error = null
        scope.launch {
            runCatching { repo.generateWallet() }
                .onSuccess {
                    mnemonic = it.mnemonic.orEmpty().trim().split(Regex("\\s+"))
                    step = OnboardingStep.PHRASE
                }
                .onFailure {
                    error = it.message ?: "Could not create the wallet."
                    step = OnboardingStep.WELCOME
                }
        }
    }

    fun restore() {
        error = null
        scope.launch {
            runCatching { repo.restoreWallet(restoreWords.joinToString(" ").trim()) }
                .onSuccess { onComplete() }
                .onFailure { error = it.message ?: "Could not restore the wallet." }
        }
    }

    Column(Modifier.fillMaxSize().background(OceanColors.bgPrimary)) {
        if (step != OnboardingStep.WELCOME && step != OnboardingStep.CREATING) {
            ProgressHeader(
                step = step,
                onBack = {
                    error = null
                    step = if (step == OnboardingStep.CONFIRM) OnboardingStep.PHRASE else OnboardingStep.WELCOME
                },
            )
        }
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState())
                .padding(start = 22.dp, end = 22.dp, top = if (step == OnboardingStep.WELCOME) 58.dp else 12.dp, bottom = 20.dp),
        ) {
            when (step) {
                OnboardingStep.WELCOME -> Welcome(error, onCreate = ::create, onRestore = { step = OnboardingStep.RESTORE })
                OnboardingStep.CREATING -> CreatingWallet()
                OnboardingStep.PHRASE -> RecoveryPhrase(mnemonic, revealed, backedUp, { revealed = true }, { backedUp = !backedUp })
                OnboardingStep.CONFIRM -> ConfirmPhrase(mnemonic, answers)
                OnboardingStep.RESTORE -> RestoreWallet(restoreWords, error)
            }
        }
        when (step) {
            OnboardingStep.PHRASE -> FooterButton("Continue", revealed && backedUp) { step = OnboardingStep.CONFIRM }
            OnboardingStep.CONFIRM -> FooterButton(
                "Go to my wallet",
                quizPositions.indices.all { answers[it] == mnemonic[quizPositions[it]] },
                onComplete,
            )
            OnboardingStep.RESTORE -> FooterButton("Restore wallet", restoreWords.all { it.isNotBlank() }, ::restore)
            else -> Unit
        }
    }
}

@Composable
private fun Welcome(error: String?, onCreate: () -> Unit, onRestore: () -> Unit) {
    Box(
        Modifier.size(44.dp).background(OceanColors.accent, RoundedCornerShape(12.dp)),
        contentAlignment = Alignment.Center,
    ) { OIcon("spark", 25, OceanColors.onAccent) }
    Spacer(Modifier.height(22.dp))
    Hero("Lightning payouts", "Get paid your mining rewards over Lightning", "Receive OCEAN rewards straight to a wallet you control — then send them anywhere.")
    Choice("spark", "Create a new wallet", "Generate a fresh recovery phrase — takes about a minute.", true, onCreate)
    Spacer(Modifier.height(10.dp))
    Choice("in", "Restore a wallet", "Enter an existing 24-word recovery phrase.", false, onRestore)
    error?.let { Note(it, OceanColors.error) }
    Note("You stay in control. Your keys never leave your device — OCEAN only learns where to send rewards.", OceanColors.success, "shield")
}

@Composable
private fun CreatingWallet() {
    Box(Modifier.fillMaxWidth().padding(top = 180.dp), contentAlignment = Alignment.Center) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            CircularProgressIndicator(color = OceanColors.accent, strokeWidth = 3.dp)
            Spacer(Modifier.height(20.dp))
            Text("Creating your wallet", style = OceanType.sheetTitle.copy(fontSize = 21.sp))
            Text("Generating your keys on this device…", style = OceanType.bodySm, modifier = Modifier.padding(top = 8.dp))
        }
    }
}

@Composable
private fun RecoveryPhrase(words: List<String>, revealed: Boolean, backedUp: Boolean, reveal: () -> Unit, toggleBackup: () -> Unit) {
    Hero("Your recovery phrase", "Write down these 24 words", "This is the only backup of your wallet. Write them on paper, in order, and keep them private.")
    Box {
        Column(verticalArrangement = Arrangement.spacedBy(7.dp)) {
            words.chunked(3).forEachIndexed { row, chunk ->
                Row(horizontalArrangement = Arrangement.spacedBy(7.dp)) {
                    chunk.forEachIndexed { col, word -> Word(row * 3 + col + 1, word, Modifier.weight(1f)) }
                }
            }
        }
        if (!revealed) {
            Column(
                Modifier.matchParentSize().background(OceanColors.bgPrimary.copy(alpha = .93f), RoundedCornerShape(8.dp))
                    .border(1.dp, OceanColors.border, RoundedCornerShape(8.dp)).clickable { reveal() },
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
            ) {
                Box(Modifier.size(38.dp).background(OceanColors.accentDim, CircleShape), contentAlignment = Alignment.Center) {
                    OIcon("shield", 20, OceanColors.accent)
                }
                Text("Tap to reveal", style = OceanType.body.copy(fontWeight = FontWeight.SemiBold), modifier = Modifier.padding(top = 9.dp))
                Text("Make sure nobody is looking", style = OceanType.bodySm, modifier = Modifier.padding(top = 3.dp))
            }
        }
    }
    if (revealed) {
        Note("Never share these words or screenshot them. Anyone who sees them can take your bitcoin.", OceanColors.warning, "warn")
        Row(Modifier.fillMaxWidth().clickable { toggleBackup() }.padding(vertical = 14.dp), verticalAlignment = Alignment.Top) {
            Box(
                Modifier.size(20.dp).background(if (backedUp) OceanColors.accent else Color.Transparent, RoundedCornerShape(6.dp))
                    .border(1.dp, if (backedUp) OceanColors.accent else OceanColors.borderStrong, RoundedCornerShape(6.dp)),
                contentAlignment = Alignment.Center,
            ) { if (backedUp) OIcon("check", 14, OceanColors.onAccent) }
            Text("I've written my phrase down and stored it safely.", style = OceanType.body.copy(color = OceanColors.fgSecondary), modifier = Modifier.padding(start = 11.dp))
        }
    }
}

@Composable
private fun ConfirmPhrase(words: List<String>, answers: MutableList<String>) {
    Hero("Quick check", "Confirm your backup", "Using the words you just wrote down, tap the right word for each position.")
    quizPositions.forEachIndexed { quizIndex, position ->
        Text("Word #${position + 1}", style = OceanType.bodySm.copy(color = OceanColors.fgTertiary), modifier = Modifier.padding(top = 13.dp, bottom = 8.dp))
        val options = listOf(words[position], decoys[quizIndex * 2], decoys[quizIndex * 2 + 1], words[(position + 5) % words.size])
        options.chunked(2).forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { option ->
                    val selected = answers[quizIndex] == option
                    val correct = option == words[position]
                    Box(
                        Modifier.weight(1f).padding(bottom = 8.dp)
                            .background(if (selected) (if (correct) OceanColors.successDim else OceanColors.errorDim) else OceanColors.bgCard, RoundedCornerShape(8.dp))
                            .border(1.dp, if (!selected) OceanColors.border else if (correct) OceanColors.success else OceanColors.error, RoundedCornerShape(8.dp))
                            .clickable { answers[quizIndex] = option }.padding(12.dp),
                        contentAlignment = Alignment.Center,
                    ) { Text(option, style = OceanType.monoSm.copy(color = if (!selected) OceanColors.fgSecondary else if (correct) OceanColors.success else OceanColors.error)) }
                }
            }
        }
    }
}

@Composable
private fun RestoreWallet(words: MutableList<String>, error: String?) {
    Hero("Restore wallet", "Enter your recovery phrase", "Type each word in order. Nothing leaves your device.")
    Note("OCEAN staff will never ask you for these words.", OceanColors.warning, "warn")
    Spacer(Modifier.height(14.dp))
    words.indices.chunked(2).forEach { row ->
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            row.forEach { index ->
                OutlinedTextField(
                    value = words[index],
                    onValueChange = { value ->
                        val pasted = value.trim().lowercase().split(Regex("\\s+"))
                        if (index == 0 && pasted.size > 1) {
                            pasted.take(words.size).forEachIndexed { pastedIndex, word ->
                                words[pastedIndex] = word
                            }
                        } else {
                            words[index] = value.trim().lowercase()
                        }
                    },
                    prefix = { Text("${index + 1}", style = OceanType.monoXs) },
                    singleLine = true,
                    textStyle = OceanType.monoSm.copy(color = OceanColors.fgPrimary),
                    keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.None, autoCorrectEnabled = false, keyboardType = KeyboardType.Text),
                    modifier = Modifier.weight(1f),
                    shape = RoundedCornerShape(8.dp),
                )
            }
        }
    }
    error?.let { Note(it, OceanColors.error) }
}

@Composable
private fun Hero(eyebrow: String, title: String, subtitle: String) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(7.dp)) {
        OIcon("spark", 12, OceanColors.accent)
        Text(eyebrow.uppercase(), style = OceanType.sectionLabel.copy(color = OceanColors.accent))
    }
    Text(title, style = OceanType.headTitle.copy(fontSize = 27.sp, lineHeight = 31.sp), modifier = Modifier.padding(top = 12.dp))
    Text(subtitle, style = OceanType.body.copy(color = OceanColors.fgTertiary, fontSize = 14.5.sp, lineHeight = 23.sp), modifier = Modifier.padding(top = 10.dp, bottom = 22.dp))
}

@Composable
private fun Choice(icon: String, title: String, subtitle: String, recommended: Boolean, onClick: () -> Unit) {
    Box {
        Row(
            Modifier.fillMaxWidth().background(OceanColors.bgCard, RoundedCornerShape(12.dp))
                .border(1.dp, OceanColors.border, RoundedCornerShape(12.dp)).clickable { onClick() }.padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(Modifier.size(38.dp).background(OceanColors.accentDim, RoundedCornerShape(10.dp)), contentAlignment = Alignment.Center) {
                OIcon(icon, 18, OceanColors.accent)
            }
            Column(Modifier.weight(1f).padding(horizontal = 13.dp)) {
                Text(title, style = OceanType.body.copy(fontSize = 14.5.sp, fontWeight = FontWeight.SemiBold))
                Text(subtitle, style = OceanType.bodySm.copy(color = OceanColors.fgTertiary, lineHeight = 17.sp), modifier = Modifier.padding(top = 3.dp))
            }
            OIcon("chevR", 17, OceanColors.fgMuted)
        }
        if (recommended) Text(
            "RECOMMENDED",
            style = OceanType.monoXs.copy(color = OceanColors.onAccent, fontSize = 8.5.sp),
            modifier = Modifier.padding(start = 14.dp).background(OceanColors.accent, RoundedCornerShape(99.dp)).padding(horizontal = 8.dp, vertical = 2.dp),
        )
    }
}

@Composable
private fun Word(index: Int, word: String, modifier: Modifier) {
    Row(
        modifier.background(OceanColors.bgCard, RoundedCornerShape(8.dp)).border(1.dp, OceanColors.borderSubtle, RoundedCornerShape(8.dp)).padding(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text("$index", style = OceanType.monoXs.copy(fontSize = 9.sp), modifier = Modifier.padding(end = 6.dp))
        Text(word, style = OceanType.monoSm.copy(color = OceanColors.fgPrimary), maxLines = 1)
    }
}

@Composable
private fun Note(text: String, color: Color, icon: String = "warn") {
    Row(
        Modifier.fillMaxWidth().padding(top = 16.dp).background(color.copy(alpha = .06f), RoundedCornerShape(8.dp))
            .border(1.dp, color.copy(alpha = .25f), RoundedCornerShape(8.dp)).padding(13.dp),
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        OIcon(icon, 16, color)
        Text(text, style = OceanType.bodySm.copy(color = OceanColors.fgSecondary, lineHeight = 19.sp))
    }
}

@Composable
private fun ProgressHeader(step: OnboardingStep, onBack: () -> Unit) {
    val progress = when (step) {
        OnboardingStep.PHRASE -> 1
        OnboardingStep.CONFIRM -> 2
        OnboardingStep.RESTORE -> 1
        else -> 0
    }
    val total = if (step == OnboardingStep.RESTORE) 1 else 2
    Row(Modifier.fillMaxWidth().padding(start = 20.dp, end = 20.dp, top = 52.dp, bottom = 10.dp), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(34.dp).clickable { onBack() }, contentAlignment = Alignment.Center) { OIcon("chevL", 16) }
        Row(Modifier.weight(1f), horizontalArrangement = Arrangement.spacedBy(5.dp)) {
            repeat(total) { i -> Box(Modifier.weight(1f).height(3.dp).background(if (i < progress) OceanColors.accent else OceanColors.borderStrong, RoundedCornerShape(99.dp))) }
        }
        Text("$progress / $total", style = OceanType.monoXs, modifier = Modifier.padding(start = 12.dp))
    }
}

@Composable
private fun FooterButton(text: String, enabled: Boolean, onClick: () -> Unit) {
    Box(Modifier.fillMaxWidth().border(1.dp, OceanColors.borderSubtle).padding(start = 20.dp, end = 20.dp, top = 14.dp, bottom = 30.dp)) {
        OButton(text, BtnVariant.PRIMARY, iconRight = "chevR", enabled = enabled, fill = true, onClick = onClick)
    }
}
