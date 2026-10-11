// Pairing codes: showing one (QR, words, countdown ring), typing one (three word slots with
// autocomplete) and scanning one (camera). Twins of CodeDisplay.svelte, CodeInput.svelte, QrScanner.svelte.
package io.github.kimzam30.nhakobeam.ui.components

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathMeasure
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.google.zxing.BarcodeFormat
import com.google.zxing.EncodeHintType
import com.google.zxing.common.BitMatrix
import com.google.zxing.qrcode.QRCodeWriter
import com.google.zxing.qrcode.decoder.ErrorCorrectionLevel
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.model.PairingCode
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.Words
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style
import kotlin.math.max
import kotlinx.coroutines.delay

/** The QR matrix for a code URI. */
fun qrMatrix(text: String): BitMatrix =
    QRCodeWriter().encode(text, BarcodeFormat.QR_CODE, 0, 0, mapOf(EncodeHintType.ERROR_CORRECTION to ErrorCorrectionLevel.M, EncodeHintType.MARGIN to 0))

/** "Expires in 10 min" / "in 45 sec". */
fun formatRemaining(ms: Long): String {
    val s = max(0L, ms / 1000)
    return if (s >= 60) "${(s + 59) / 60} min" else "$s sec"
}

/**
 * The code this device shows: a QR code drawn as dots with rounded finder squares, a ring around
 * it that counts down the 10 minutes (warning color in the last 20 seconds), the three words, and Copy.
 */
@Composable
fun CodeDisplay(app: AppState, code: PairingCode, modifier: Modifier = Modifier) {
    val c = Nhako.colors
    val context = LocalContext.current
    val matrix = remember(code.uri) { qrMatrix(code.uri) }
    var now by remember { mutableLongStateOf(System.currentTimeMillis()) }
    LaunchedEffect(code.expiresAt) {
        while (true) {
            now = System.currentTimeMillis()
            delay(250)
        }
    }
    val left = code.expiresAt - now
    val expired = left <= 0
    val soon = left in 1..20_000
    val fraction by animateFloatAsState((left / 600_000f).coerceIn(0f, 1f), tween(250), label = "ring")
    val ringColor = if (soon) c.warning else c.accent
    val appear = remember(code.uri) { Animatable(0.92f) }
    LaunchedEffect(code.uri) { appear.animateTo(1f, NhakoMotion.bouncy.spec()) }

    Column(modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(NhakoSpace.s5)) {
        Box(
            Modifier.size(232.dp).graphicsLayer { scaleX = appear.value; scaleY = appear.value; alpha = if (expired) 0.35f else 1f }
                .semantics { contentDescription = "QR code for ${code.words.joinToString(" ")}" },
            contentAlignment = Alignment.Center,
        ) {
            Canvas(Modifier.size(232.dp)) {
                val stroke = 3.dp.toPx()
                val inset = stroke / 2
                val r = 28.dp.toPx()
                drawRoundRect(c.beamTrack, Offset(inset, inset), Size(size.width - stroke, size.height - stroke), CornerRadius(r), style = Stroke(stroke))
                // The countdown: the part of the outline that's left, starting at the top middle.
                val path = Path().apply {
                    addRoundRect(androidx.compose.ui.geometry.RoundRect(inset, inset, size.width - inset, size.height - inset, CornerRadius(r)))
                }
                val measure = PathMeasure().apply { setPath(path, false) }
                val total = measure.length
                val seg = Path()
                val start = total * 0.125f
                val len = total * fraction
                if (start + len <= total) measure.getSegment(start, start + len, seg, true)
                else {
                    measure.getSegment(start, total, seg, true)
                    measure.getSegment(0f, start + len - total, seg, true)
                }
                drawPath(seg, ringColor, style = Stroke(stroke, cap = StrokeCap.Round))
            }
            Box(Modifier.size(200.dp).clip(RoundedCornerShape(22.dp)).background(Color.White).padding(14.dp)) {
                QrDots(matrix, c.accent)
            }
        }
        BasicText(
            code.words.joinToString("  "),
            style = NhakoType.codeWords.style(c.textPrimary).copy(textAlign = TextAlign.Center),
            modifier = Modifier.semantics { contentDescription = "Code: ${code.words.joinToString(", ")}" },
        )
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            PhIcon("timer", size = 16.dp, tint = if (soon) c.warning else c.textSecondary)
            Spacer(Modifier.width(NhakoSpace.s2))
            BasicText(
                if (expired) t("code.show.expired") else t("code.show.expiresIn", "time" to formatRemaining(left)),
                Modifier.weight(1f),
                style = NhakoType.footnote.style(if (soon) c.warning else c.textSecondary),
            )
            if (expired) {
                NButton(t("code.show.new"), { app.newCode() }, size = ButtonSize.Sm, icon = "arrows-clockwise")
            } else {
                NButton(t("code.show.copy"), {
                    val cm = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                    cm.setPrimaryClip(ClipData.newPlainText("Nhako Beam code", code.words.joinToString(" ")))
                    app.toast(t("code.show.copied"))
                }, variant = ButtonVariant.Secondary, size = ButtonSize.Sm, icon = "copy")
            }
        }
    }
}

/** QR modules as dots, finder patterns as rounded squares with an accent center. */
@Composable
fun QrDots(m: BitMatrix, accent: Color, modifier: Modifier = Modifier) {
    Canvas(modifier.size(172.dp)) {
        val n = m.width
        val cell = size.width / n
        fun inFinder(x: Int, y: Int) = (x < 7 && y < 7) || (x >= n - 7 && y < 7) || (x < 7 && y >= n - 7)
        for (y in 0 until n) for (x in 0 until n) {
            if (m[x, y] && !inFinder(x, y)) drawCircle(Color(0xFF121418), radius = cell * 0.42f, center = Offset((x + 0.5f) * cell, (y + 0.5f) * cell))
        }
        for ((fx, fy) in listOf(0 to 0, n - 7 to 0, 0 to n - 7)) {
            val o = Offset(fx * cell, fy * cell)
            drawRoundRect(Color(0xFF121418), o + Offset(cell * 0.5f, cell * 0.5f), Size(cell * 6, cell * 6), CornerRadius(cell * 1.8f), style = Stroke(cell))
            drawRoundRect(accent, o + Offset(cell * 2, cell * 2), Size(cell * 3, cell * 3), CornerRadius(cell * 0.9f))
        }
    }
}

/** What CodeInput exposes: the three words, and a way to shake a slot after a wrong code. */
@Stable
class CodeInputState {
    val words = mutableListOf("", "", "").let { androidx.compose.runtime.mutableStateListOf(*it.toTypedArray()) }
    internal var shakeSlot by mutableIntStateOf(-2)
    internal var shakeSeq by mutableIntStateOf(0)
    val complete: Boolean get() = words.all { it.trim().length > 1 }

    /** Shakes slot [index], or all three when null. */
    fun shake(index: Int?) {
        shakeSlot = index ?: -1
        shakeSeq++
    }

    /** Pasting "river lantern seven" (or with dashes) into any slot fills all three. */
    fun set(index: Int, value: String) {
        val parts = value.trim().lowercase().split(Regex("[\\s,.-]+")).filter { it.isNotEmpty() }
        if (parts.size >= 3) {
            for (i in 0..2) words[i] = parts[i]
        } else {
            words[index] = value.lowercase().replace(" ", "")
        }
    }
}

/** Three word slots with a check when a word is real, and suggestions under the slot being typed in. */
@Composable
fun CodeInput(state: CodeInputState, enabled: Boolean, onSubmit: () -> Unit, modifier: Modifier = Modifier) {
    val c = Nhako.colors
    val focus = remember { List(3) { FocusRequester() } }
    var active by remember { mutableIntStateOf(0) }
    Column(modifier, verticalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
        Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
            for (i in 0..2) {
                val shake = remember { Animatable(0f) }
                LaunchedEffect(state.shakeSeq) {
                    if (state.shakeSeq > 0 && (state.shakeSlot == -1 || state.shakeSlot == i)) {
                        for (x in listOf(-8f, 7f, -5f, 3f, 0f)) shake.animateTo(x, tween(55))
                    }
                }
                val word = state.words[i]
                val valid = Words.isWord(word)
                NTextField(
                    value = word,
                    onChange = { v ->
                        if (v.endsWith(" ") && Words.isWord(v.trim()) && i < 2) {
                            state.set(i, v.trim())
                            focus[i + 1].requestFocus()
                        } else {
                            state.set(i, v)
                        }
                    },
                    modifier = Modifier.weight(1f).focusRequester(focus[i]).onFocusChanged { if (it.isFocused) active = i }.graphicsLayer { translationX = shake.value * density },
                    placeholder = listOf("first", "second", "third")[i],
                    imeAction = if (i < 2) ImeAction.Next else ImeAction.Go,
                    onDone = { if (i < 2) focus[i + 1].requestFocus() else if (state.complete) onSubmit() },
                    style = NhakoType.headline.style(c.textPrimary),
                    trailing = if (valid) ({ PhIcon("check-circle", size = 16.dp, tint = c.accent, weight = IconWeight.Fill) }) else null,
                )
            }
        }
        val suggestions = Words.complete(state.words.getOrElse(active) { "" })
        if (enabled && suggestions.isNotEmpty()) {
            Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s2)) {
                for (s in suggestions) {
                    BasicText(
                        s,
                        Modifier.clip(RoundedCornerShape(NhakoRadius.full)).background(c.accentSubtle).clickable(role = Role.Button) {
                            state.words[active] = s
                            if (active < 2) focus[active + 1].requestFocus()
                        }.padding(horizontal = NhakoSpace.s4, vertical = NhakoSpace.s2),
                        style = NhakoType.callout.style(c.accentText).copy(fontWeight = FontWeight(500)),
                    )
                }
            }
        }
    }
    LaunchedEffect(Unit) { runCatching { focus[0].requestFocus() } }
}
