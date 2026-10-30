// The launch intro: after the system splash screen (res/drawable/splash_bolt.xml), the voxel bolt
// drops onto its spot on the same brand blue, lands with a squash and blinks, then the blue fades
// into the app. The Compose twin of desktop/src/components/SplashIntro.svelte. Motion:
// docs/design/motion-spec.md, section 4.14.
package io.github.kimzam30.nhakobeam.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.design.NhakoBrand
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoSound
import io.github.kimzam30.nhakobeam.stage.Mascot3D
import io.github.kimzam30.nhakobeam.stage.MascotState
import io.github.kimzam30.nhakobeam.ui.LocalSound
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import kotlinx.coroutines.delay

/** The bolt's own fall and landing take this long; then the blue fades. */
private const val LAND_MS = 900L
private const val FADE_MS = 240

/**
 * Full motion, about 1.1 s: the bolt falls, lands with a squash, blinks, then the blue fades.
 * Reduced motion: a 200 ms fade. Touches are held back until it's gone.
 */
@Composable
fun SplashIntro(onDone: () -> Unit, modifier: Modifier = Modifier) {
    val reduced = Nhako.reducedMotion
    val done by rememberUpdatedState(onDone)
    val cover = remember { Animatable(1f) }
    val play = LocalSound.current
    LaunchedEffect(reduced) {
        if (reduced) {
            play(NhakoSound.Intro)
            cover.animateTo(0f, tween(200, easing = NhakoMotion.Easing.exit))
        } else {
            // The landing click.
            delay(420)
            play(NhakoSound.Intro)
            delay(LAND_MS - 420)
            cover.animateTo(0f, tween(FADE_MS, easing = NhakoMotion.Easing.exit))
        }
        done()
    }

    Box(
        modifier
            .fillMaxSize()
            .clearAndSetSemantics {}
            .pointerInput(Unit) { awaitEachGesture { awaitFirstDown().consume() } }
            .graphicsLayer { alpha = cover.value }
            .background(NhakoBrand.blue),
        contentAlignment = Alignment.Center,
    ) {
        Mascot3D(MascotState.Idle, 288.dp, "Nhako Beam", intro = true)
    }
}
