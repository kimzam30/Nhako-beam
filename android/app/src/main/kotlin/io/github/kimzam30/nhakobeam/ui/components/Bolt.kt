// The bolt from the logo, as motion: its shape, the morph into a check, the crack, the arc
// between two points, and the sparks thrown when something commits. "Charge, zap, settle".
// The Compose twin of desktop/src/lib/motion/bolt.ts and Bolt.svelte. Spec: motion-spec.md, 4.15.
package io.github.kimzam30.nhakobeam.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.keyframes
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.Dp
import io.github.kimzam30.nhakobeam.design.NhakoBrand
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.hypot
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/** The bolt's seven corners, in a 1024 box: the vector bolt from the first logo, kept as the
 * motion shape now that the logo is the pixel mascot. */
val BOLT_POINTS = listOf(
    676.16f to 161.6f, 829.76f to 244.16f, 608.96f to 464.96f, 743.36f to 503.36f,
    194.24f to 862.4f, 436.16f to 574.4f, 301.76f to 536f,
)

/** A thick check with the same seven corners in the same order, so the bolt morphs into it. */
val CHECK_POINTS = listOf(
    774f to 248f, 866f to 332f, 648f to 570f, 430f to 807f,
    166f to 544f, 254f to 456f, 430f to 633f,
)

/** The crack runs through the bolt's own zig. */
private val CRACK_A = Offset(0f, 640f)
private val CRACK_B = Offset(1024f, 380f)

/** The corners between the bolt (0) and the check (1), in the 1024 box. */
fun boltCorners(morph: Float): List<Pair<Float, Float>> {
    val m = morph.coerceIn(0f, 1f)
    return BOLT_POINTS.mapIndexed { i, (x, y) ->
        val (cx, cy) = CHECK_POINTS[i]
        (x + (cx - x) * m) to (y + (cy - y) * m)
    }
}

/** The shape as a Path in the 1024 box. */
fun boltPath(morph: Float = 0f): Path = Path().apply {
    boltCorners(morph).forEachIndexed { i, (x, y) -> if (i == 0) moveTo(x, y) else lineTo(x, y) }
    close()
}

/**
 * A jagged lightning line from a to b: corners pushed off the straight line, smaller near the
 * ends so it leaves and lands cleanly. Same zig as the desktop's `arcPath`.
 */
fun arcPoints(a: Offset, b: Offset, seed: Int = 0, amp: Float = 7f): List<Offset> {
    val dx = b.x - a.x
    val dy = b.y - a.y
    val len = hypot(dx, dy).takeIf { it > 0f } ?: 1f
    val nx = -dy / len
    val ny = dx / len
    val steps = (len / 22f).roundToInt().coerceIn(3, 8)
    val pts = mutableListOf(a)
    for (i in 1 until steps) {
        val f = i.toFloat() / steps
        val side = if ((i + seed) % 2 != 0) 1f else -1f
        val k = amp * sin(PI.toFloat() * f) * (0.7f + 0.3f * (((i * 7 + seed * 3) % 5) / 4f))
        pts += Offset(a.x + dx * f + nx * k * side, a.y + dy * f + ny * k * side)
    }
    pts += b
    return pts
}

/** Draws a bolt (or the shape part way to a check) centered on [center], [sizePx] across. */
fun DrawScope.drawBolt(center: Offset, sizePx: Float, color: Color = NhakoBrand.yellow, shadow: Color? = NhakoBrand.navy, morph: Float = 0f, rotation: Float = 0f, alpha: Float = 1f) {
    val path = boltPath(morph)
    // The shape's own box is about 784 units across (120..904), like the desktop's viewBox.
    val k = sizePx / 784f
    translate(center.x - 512f * k, center.y - 512f * k) {
        rotate(rotation, pivot = Offset(512f * k, 512f * k)) {
            scale(k, k, pivot = Offset.Zero) {
                if (shadow != null) translate(26f, 32f) {
                    drawPath(path, shadow, alpha = alpha * (1f - morph))
                    drawPath(path, shadow, alpha = alpha * (1f - morph), style = Stroke(22f, join = StrokeJoin.Round))
                }
                drawPath(path, color, alpha = alpha)
                drawPath(path, color, alpha = alpha, style = Stroke(22f, join = StrokeJoin.Round))
            }
        }
    }
}

enum class BoltMode { Bolt, Check, Cracked, Asleep }

/**
 * The logo bolt as a living icon. [mode]: bolt, check (it morphs into a check), cracked (it
 * splits along its zig), asleep (dim, breathes once). [strike] drops it in with sparks first.
 * Bumping [twitch] gives it one small shake. [brand] = yellow with the dark blue offset; otherwise
 * it's drawn flat in [tint]. A brand bolt turning into a check takes [tint].
 */
@Composable
fun Bolt(size: Dp, modifier: Modifier = Modifier, mode: BoltMode = BoltMode.Bolt, brand: Boolean = true, tint: Color = Nhako.colors.textPrimary, strike: Boolean = false, twitch: Int = 0) {
    val reduced = Nhako.reducedMotion
    val morph = remember { Animatable(0f) }
    val split = remember { Animatable(0f) }
    val drop = remember { Animatable(if (strike && !reduced) 0f else 1f) }
    val wiggle = remember { Animatable(0f) }
    val breathe = remember { Animatable(if (mode == BoltMode.Asleep && !reduced) 0f else 1f) }
    var sparks by remember { mutableStateOf(0) }
    var landed by remember { mutableStateOf(!strike || reduced) }

    LaunchedEffect(Unit) {
        if (!landed) {
            drop.animateTo(1f, NhakoMotion.bouncy.spec())
            sparks++
            if (mode == BoltMode.Check) delay(120)
            landed = true
        }
    }
    LaunchedEffect(mode, landed) {
        if (!landed) return@LaunchedEffect
        val m = if (mode == BoltMode.Check) 1f else 0f
        val s = if (mode == BoltMode.Cracked) 1f else 0f
        if (reduced) {
            morph.snapTo(m)
            split.snapTo(s)
        } else {
            launch { morph.animateTo(m, NhakoMotion.snappy.spec()) }
            split.animateTo(s, NhakoMotion.snappy.spec())
        }
    }
    LaunchedEffect(mode) {
        if (mode == BoltMode.Asleep && !reduced) {
            breathe.snapTo(0f)
            delay(200)
            breathe.animateTo(1f, tween(1600, easing = LinearEasing))
        }
    }
    var lastTwitch by remember { mutableStateOf(twitch) }
    LaunchedEffect(twitch) {
        if (twitch == lastTwitch) return@LaunchedEffect
        lastTwitch = twitch
        if (reduced) return@LaunchedEffect
        wiggle.animateTo(0f, keyframes { durationMillis = 300; 1f at 70; -0.7f at 150; 0f at 300 })
    }

    val disabled = Nhako.colors.textDisabled
    Canvas(
        modifier.size(size).sparks(sparks, count = 4).graphicsLayer {
            val b = breathe.value
            // Breathes once: up a little, then rests slightly dim.
            val swell = if (mode == BoltMode.Asleep) (if (b < 0.45f) 0.94f + 0.12f * (b / 0.45f) else 1.06f - 0.06f * ((b - 0.45f) / 0.55f)) else 1f
            scaleX = swell
            scaleY = swell
            alpha = min(1f, drop.value * 1.6f) * (if (mode == BoltMode.Asleep) (0.4f + 0.6f * min(1f, b / 0.45f)) - 0.2f * max(0f, (b - 0.45f) / 0.55f) else 1f)
        },
    ) {
        val px = this.size.minDimension
        val d = drop.value
        val m = morph.value
        val color = when {
            mode == BoltMode.Asleep -> disabled
            brand && m < 0.35f -> NhakoBrand.yellow
            else -> tint
        }
        val shadow = if (brand && mode != BoltMode.Asleep) NhakoBrand.navy else null
        val c = Offset(this.size.width / 2, this.size.height / 2 - (1 - d) * px * 0.33f)
        val s = px * (0.55f + 0.45f * d)
        val rot = wiggle.value * 9f
        val sp = split.value
        if (sp <= 0.002f) {
            drawBolt(c, s, color, shadow, m, rot)
        } else {
            // Cracked: two halves, cut along the zig, pushed apart and turned a little.
            val k = s / 784f
            val o = Offset(c.x - 512f * k, c.y - 512f * k)
            fun cut(top: Boolean) = Path().apply {
                val a = o + CRACK_A * k
                val b = o + CRACK_B * k
                if (top) { moveTo(o.x - px, o.y - px); lineTo(o.x + 2 * px, o.y - px); lineTo(b.x + px, b.y - px * 0.25f); lineTo(a.x - px, a.y + px * 0.25f) } else { moveTo(a.x - px, a.y + px * 0.25f); lineTo(b.x + px, b.y - px * 0.25f); lineTo(o.x + 2 * px, o.y + 2 * px); lineTo(o.x - px, o.y + 2 * px) }
                close()
            }
            for (top in listOf(true, false)) {
                val dir = if (top) -1f else 1f
                // Moved first, then cut, so the cut travels with its half (as on desktop).
                translate(dir * 30f * k * sp, dir * 44f * k * sp) {
                    rotate(dir * 9f * sp, pivot = c) {
                        clipPath(cut(top)) { drawBolt(c, s, color, shadow, m, rot) }
                    }
                }
            }
        }
    }
}

/**
 * Throws a handful of tiny bolts out from the middle of this element each time [fire] goes up.
 * Drawn on top without clipping; about 300 ms; transform and alpha only. Reduced motion: none.
 */
fun Modifier.sparks(fire: Int, count: Int = NhakoMotion.Bolt.sparkCount, reach: Float = NhakoMotion.Bolt.sparkReach, sizeDp: Float = NhakoMotion.Bolt.sparkSize, toward: Float? = null, at: Offset? = null): Modifier = composed {
    val reduced = Nhako.reducedMotion
    val t = remember { Animatable(1f) }
    var turn by remember { mutableStateOf(0f) }
    LaunchedEffect(fire) {
        if (fire <= 0 || reduced) return@LaunchedEffect
        turn = (fire * 0.9f) % (2 * PI.toFloat())
        t.snapTo(0f)
        t.animateTo(1f, tween(NhakoMotion.Bolt.sparkMs, easing = LinearEasing))
    }
    drawWithContent {
        drawContent()
        val p = t.value
        if (p >= 1f || reduced) return@drawWithContent
        // Out fast and full size, then shrink and fade the last stretch (as on desktop).
        val out = if (p < 0.6f) NhakoMotion.Easing.enter.transform(p / 0.6f) * 0.8f else 0.8f + 0.2f * (p - 0.6f) / 0.4f
        val sc = if (p < 0.6f) 0.5f + 0.5f * (p / 0.6f) else 1f - 0.4f * (p - 0.6f) / 0.4f
        val alpha = if (p < 0.6f) 1f else 1f - (p - 0.6f) / 0.4f
        val origin = at ?: Offset(size.width / 2, size.height / 2)
        for (i in 0 until count) {
            val a = if (toward == null) turn + i.toFloat() / count * 2 * PI.toFloat() else toward + (i.toFloat() / max(1, count - 1) - 0.5f) * (2 * PI.toFloat() / 3)
            val r = reach * density * (0.8f + 0.2f * ((i * 5) % 3) / 2f) * out
            val sz = sizeDp * density * (0.75f + 0.25f * ((i * 3) % 4) / 3f) * sc
            drawBolt(Offset(origin.x + cos(a) * r, origin.y + sin(a) * r), sz, rotation = a * 180f / PI.toFloat() + 90f, alpha = alpha)
        }
    }
}
