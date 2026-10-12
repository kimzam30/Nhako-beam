// The launch intro: picks up exactly where the system splash screen (res/drawable/splash_bolt.xml)
// leaves off, plays a short "charge" on the bolt, then fades into the app. The Compose twin of
// desktop/src/components/SplashIntro.svelte. Motion: docs/design/motion-spec.md, section 4.14.
package io.github.kimzam30.nhakobeam.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.PathParser
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.design.NhakoBrand
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoSound
import io.github.kimzam30.nhakobeam.ui.LocalSound
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/** The bolt from docs/brand/logo/svg/nhako-beam-icon.svg, in its 1024 unit box. */
private const val BOLT = "M648.8,220 L776.8,288.8 L592.8,472.8 L704.8,504.8 L247.2,804 L448.8,564 L336.8,532 Z"
/** Where the dark blue copy sits behind it. */
private val SHADOW = Offset(18f, 22f)

/**
 * Full motion, about 820 ms: the bolt presses in (its shadow tucks under it), springs back out on
 * the bouncy spring while a ring pulses outward, then the blue fades away as the bolt shrinks a
 * little. Reduced motion: a 200 ms fade. Touches are held back until it's gone.
 */
@Composable
fun SplashIntro(onDone: () -> Unit, modifier: Modifier = Modifier) {
    val reduced = Nhako.reducedMotion
    val done by rememberUpdatedState(onDone)
    val bolt = remember { PathParser().parsePathString(BOLT).toPath() }
    // 1 = resting like the system splash; lower = pressed in.
    val lift = remember { Animatable(1f) }
    val grow = remember { Animatable(1f) }
    val ring = remember { Animatable(0f) }
    val cover = remember { Animatable(1f) }

    val play = LocalSound.current
    LaunchedEffect(reduced) {
        // Its click lands as the bolt presses in (175 ms), so it starts with the intro.
        play(NhakoSound.Intro)
        if (reduced) {
            cover.animateTo(0f, tween(200, easing = NhakoMotion.Easing.exit))
            done()
            return@LaunchedEffect
        }
        coroutineScope {
            launch { lift.animateTo(0f, tween(180, easing = NhakoMotion.Easing.standard)) }
            grow.animateTo(NhakoMotion.Scale.enterFrom, tween(180, easing = NhakoMotion.Easing.standard))
            launch { ring.animateTo(1f, tween(520, easing = NhakoMotion.Easing.enter)) }
            launch { lift.animateTo(1f, NhakoMotion.bouncy.spec()) }
            launch { grow.animateTo(1f, NhakoMotion.bouncy.spec()) }
            delay(400)
            launch { grow.animateTo(0.82f, tween(240, easing = NhakoMotion.Easing.exit)) }
            cover.animateTo(0f, tween(240, easing = NhakoMotion.Easing.exit))
        }
        done()
    }

    Box(
        modifier
            .fillMaxSize()
            .clearAndSetSemantics {}
            .pointerInput(Unit) { awaitEachGesture { awaitFirstDown().consume() } }
            .graphicsLayer { alpha = cover.value },
        contentAlignment = Alignment.Center,
    ) {
        Canvas(Modifier.fillMaxSize()) { drawRect(NhakoBrand.blue) }
        // The same 288 dp box as the system splash icon, so the hand-over doesn't move a pixel.
        Canvas(Modifier.size(288.dp)) {
            val unit = size.width / 288f
            val center = Offset(size.width / 2f, size.height / 2f)
            val r = ring.value
            if (r > 0f && r < 1f) {
                drawCircle(
                    color = Color.White.copy(alpha = 0.45f * (1f - r)),
                    radius = (64f + 104f * r) * unit,
                    center = center,
                    style = Stroke(width = 3f * unit),
                )
            }
            scale(grow.value, pivot = center) {
                // splash_bolt.xml: scale 0.2226 and move 30 dp, in a 288 dp box.
                translate(30f * unit, 30f * unit) {
                    scale(0.2226f * unit, pivot = Offset.Zero) {
                        val round = Stroke(width = 12f, join = StrokeJoin.Round)
                        translate(SHADOW.x * lift.value, SHADOW.y * lift.value) {
                            drawPath(bolt, NhakoBrand.navy)
                            drawPath(bolt, NhakoBrand.navy, style = round)
                        }
                        drawPath(bolt, NhakoBrand.yellow)
                        drawPath(bolt, NhakoBrand.yellow, style = round)
                    }
                }
            }
        }
    }
}
