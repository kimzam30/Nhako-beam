package io.github.kimzam30.nhakobeam.stage

import android.content.Context
import android.util.Log
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.compose.animation.core.withInfiniteAnimationFrameMillis
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import io.github.kimzam30.nhakobeam.ui.components.Bolt
import io.github.kimzam30.nhakobeam.ui.components.BoltMode
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import kotlinx.coroutines.delay

/** The mascot's model and stage, read from the assets once per process. */
object MascotAssets {
    @Volatile private var cached: MascotModel? = null
    @Volatile private var failed = false

    fun get(context: Context): MascotModel? {
        cached?.let { return it }
        if (failed) return null
        return synchronized(this) {
            cached ?: runCatching {
                val assets = context.applicationContext.assets
                MascotModel.load(assets.open("mascot.glb").use { it.readBytes() }, assets.open("mascot.json").use { it.reader().readText() })
            }.onFailure {
                failed = true
                Log.w("nhako", "The 3D mascot couldn't load; the flat bolt stays: ${it.message}")
            }.getOrNull()?.also { cached = it }
        }
    }
}

/**
 * Draws [scene] every frame while something moves, and a few times a second otherwise (with
 * reduced motion nothing moves, so it only catches changes).
 */
@Composable
fun StageCanvas(scene: StageScene, modifier: Modifier, paused: Boolean = false) {
    val c = Nhako.colors
    scene.reduced = Nhako.reducedMotion
    scene.floorColor = c.fill.toArgb()
    scene.ringColor = c.separator.toArgb()
    scene.accent = c.accent.toArgb()
    var tick by remember { mutableLongStateOf(0L) }
    // Held still while something covers it (a sheet). An endless animation, so tests (which
    // don't run those) draw it once and go on.
    LaunchedEffect(scene, paused) {
        if (paused) return@LaunchedEffect
        while (true) {
            withInfiniteAnimationFrameMillis { tick = it }
            if (!scene.moving) delay(250) else if (scene.calm) delay(17)
        }
    }
    Canvas(modifier) {
        val now = tick
        drawIntoCanvas { scene.drawSafely(it.nativeCanvas, size.width, size.height, now) }
    }
}

/**
 * The voxel mascot on its own (the splash, empty History, pairing). Where the model can't load,
 * the flat bolt stands in. motion-spec.md section 4.18.
 */
@Composable
fun Mascot3D(
    mood: MascotState,
    height: Dp,
    label: String,
    modifier: Modifier = Modifier,
    width: Dp = height,
    pair: Boolean = false,
    intro: Boolean = false,
) {
    val context = LocalContext.current
    val model = remember { MascotAssets.get(context) }
    Box(modifier.size(width, height).semantics { contentDescription = label }, contentAlignment = Alignment.Center) {
        if (model == null) {
            val flat = when (mood) {
                MascotState.Asleep -> BoltMode.Asleep
                MascotState.Failed -> BoltMode.Cracked
                MascotState.Done -> BoltMode.Check
                else -> BoltMode.Bolt
            }
            Bolt(height * 0.42f, mode = flat)
            return@Box
        }
        val scene = remember(model, pair, intro) {
            StageScene(
                model,
                StageCamera(floatArrayOf(0f, 2.6f, if (pair) 10.5f else 8.6f), floatArrayOf(0f, 1.45f, 0f), 30f),
                home = false, mascotScale = 1f, pair = pair, intro = intro,
            )
        }
        scene.mood = mood
        StageCanvas(scene, Modifier.size(width, height))
    }
}
