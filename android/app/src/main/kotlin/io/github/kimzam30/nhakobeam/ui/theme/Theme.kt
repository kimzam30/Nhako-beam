package io.github.kimzam30.nhakobeam.ui.theme

import android.annotation.SuppressLint
import android.provider.Settings
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.remember
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.foundation.Canvas
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.BlendMode
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Paint
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathFillType
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.graphics.vector.PathParser
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.R
import io.github.kimzam30.nhakobeam.design.NhakoColors
import io.github.kimzam30.nhakobeam.design.NhakoIcons
import io.github.kimzam30.nhakobeam.design.NhakoPalette
import io.github.kimzam30.nhakobeam.design.NhakoTypeToken
import io.github.kimzam30.nhakobeam.state.MotionPref
import io.github.kimzam30.nhakobeam.state.ThemePref

/** Inter (variable), bundled so text looks the same on every phone. */
@OptIn(androidx.compose.ui.text.ExperimentalTextApi::class)
val Inter = FontFamily(
    listOf(400, 500, 600, 650, 700).map { w ->
        Font(R.font.inter, FontWeight(w), variationSettings = FontVariation.Settings(FontVariation.weight(w)))
    },
)

@Immutable
data class NhakoLook(val colors: NhakoColors, val dark: Boolean, val reducedMotion: Boolean)

val LocalLook = staticCompositionLocalOf { NhakoLook(NhakoPalette.Light, dark = false, reducedMotion = false) }

/** Nothing has happened for a while (AppState.still): what moves for its own sake holds still. */
val LocalStill = compositionLocalOf { false }

/** Shortcuts: `Nhako.colors.accent`, `Nhako.reducedMotion`. */
object Nhako {
    val colors: NhakoColors @Composable get() = LocalLook.current.colors
    val dark: Boolean @Composable get() = LocalLook.current.dark
    val reducedMotion: Boolean @Composable get() = LocalLook.current.reducedMotion
    val still: Boolean @Composable get() = LocalStill.current
}

/** System "Remove animations" (animator duration scale 0) counts as asking for reduced motion. */
@SuppressLint("ComposableNaming")
@Composable
private fun systemReducedMotion(): Boolean {
    val context = LocalContext.current
    return remember {
        runCatching { Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f }.getOrDefault(false)
    }
}

@Composable
fun NhakoTheme(theme: ThemePref = ThemePref.System, motion: MotionPref = MotionPref.System, content: @Composable () -> Unit) {
    val dark = when (theme) {
        ThemePref.System -> isSystemInDarkTheme()
        ThemePref.Light -> false
        ThemePref.Dark -> true
    }
    val reduced = when (motion) {
        MotionPref.System -> systemReducedMotion()
        MotionPref.Full -> false
        MotionPref.Reduced -> true
    }
    val look = NhakoLook(if (dark) NhakoPalette.Dark else NhakoPalette.Light, dark, reduced)
    CompositionLocalProvider(LocalLook provides look, content = content)
}

/** A type token as a TextStyle in Inter. */
fun NhakoTypeToken.style(color: Color = Color.Unspecified) =
    TextStyle(fontFamily = Inter, fontSize = size, lineHeight = lineHeight, fontWeight = weight, letterSpacing = tracking, color = color)

/** Kept so existing callers compile. Style B has one drawing per icon: active is shown by color, not weight. */
enum class IconWeight { Duotone, Fill, Bold }

/** An icon's paths, parsed once. */
private class ParsedIcon(val solid: List<Path>, val bold: List<Pair<Path, Float>>, val knock: List<Pair<Path, Float>>, val top: List<Pair<Path, Float>>, val topFill: List<Path>)

private val parsedIcons = HashMap<String, ParsedIcon?>()

private fun parsed(name: String): ParsedIcon? = parsedIcons.getOrPut(name) {
    val g = NhakoIcons.all[name] ?: return@getOrPut null
    fun path(d: String, evenOdd: Boolean = false) = PathParser().parsePathString(d).toPath().also { if (evenOdd) it.fillType = PathFillType.EvenOdd }
    ParsedIcon(
        g.solid.map { path(it, evenOdd = true) },
        g.bold.map { path(it.d) to it.w },
        g.knock.map { path(it.d) to it.w },
        g.top.map { path(it.d) to it.w },
        g.topFill.map { path(it) },
    )
}

/**
 * An icon from the Nhako Beam set (style B, design/icons/icons.mjs) by its name ("paper-plane-tilt"),
 * in [tint]. The same drawings as the desktop app: solid shapes, cut-outs, and from 20 dp up a navy
 * copy 1 px down and right. A blue or white icon gets the deeper navy, so the offset still shows.
 */
@Composable
fun PhIcon(name: String, modifier: Modifier = Modifier, size: Dp = 20.dp, tint: Color = Nhako.colors.textPrimary, @Suppress("UNUSED_PARAMETER") weight: IconWeight = IconWeight.Duotone) {
    val icon = parsed(name) ?: return
    val deep = tint == Nhako.colors.accent || tint == Color.White
    val offsetColor = Color(if (deep) NhakoIcons.OFFSET_DEEP else NhakoIcons.OFFSET_NAVY)
    val withOffset = size >= 20.dp
    Canvas(modifier.size(size)) {
        val unit = this.size.minDimension / 24f
        // Each copy is drawn in its own layer, so its cut-outs clear only that copy.
        fun layer(color: Color, shiftPx: Float) {
            drawIntoCanvas { canvas ->
                canvas.saveLayer(Rect(-4f, -4f, this.size.width + 4f, this.size.height + 4f), Paint())
                withTransform({ translate(shiftPx, shiftPx); scale(unit, unit, pivot = Offset.Zero) }) {
                    icon.solid.forEach { drawPath(it, color) }
                    icon.bold.forEach { (p, w) -> drawPath(p, color, style = Stroke(w, cap = StrokeCap.Round, join = StrokeJoin.Round)) }
                    icon.knock.forEach { (p, w) -> drawPath(p, Color.Black, style = Stroke(w, cap = StrokeCap.Round, join = StrokeJoin.Round), blendMode = BlendMode.Clear) }
                    icon.topFill.forEach { drawPath(it, color) }
                    icon.top.forEach { (p, w) -> drawPath(p, color, style = Stroke(w, cap = StrokeCap.Round, join = StrokeJoin.Round)) }
                }
                canvas.restore()
            }
        }
        if (withOffset) layer(offsetColor, 1f)
        layer(tint, 0f)
    }
}
