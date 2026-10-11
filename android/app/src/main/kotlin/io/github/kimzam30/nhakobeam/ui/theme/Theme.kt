package io.github.kimzam30.nhakobeam.ui.theme

import android.annotation.SuppressLint
import android.provider.Settings
import androidx.compose.foundation.Image
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.R
import io.github.kimzam30.nhakobeam.design.NhakoColors
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

/** Shortcuts: `Nhako.colors.accent`, `Nhako.reducedMotion`. */
object Nhako {
    val colors: NhakoColors @Composable get() = LocalLook.current.colors
    val dark: Boolean @Composable get() = LocalLook.current.dark
    val reducedMotion: Boolean @Composable get() = LocalLook.current.reducedMotion
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

enum class IconWeight(val suffix: String) { Duotone("duotone"), Fill("fill"), Bold("bold") }

private val iconIds = HashMap<String, Int>()

/** A Phosphor icon by its name ("paper-plane-tilt"), tinted. Falls back to duotone when a weight isn't bundled. */
@SuppressLint("DiscouragedApi", "LocalContextGetResourceValueCall")
@Composable
fun PhIcon(name: String, modifier: Modifier = Modifier, size: Dp = 20.dp, tint: Color = Nhako.colors.textPrimary, weight: IconWeight = IconWeight.Duotone) {
    val context = LocalContext.current
    fun id(w: IconWeight) = iconIds.getOrPut("$name/${w.suffix}") {
        context.resources.getIdentifier("ph_${name.replace('-', '_')}_${w.suffix}", "drawable", context.packageName)
    }
    val res = id(weight).takeIf { it != 0 } ?: id(IconWeight.Duotone)
    if (res == 0) return
    Image(painterResource(res), contentDescription = null, modifier = modifier.size(size), colorFilter = ColorFilter.tint(tint))
}
