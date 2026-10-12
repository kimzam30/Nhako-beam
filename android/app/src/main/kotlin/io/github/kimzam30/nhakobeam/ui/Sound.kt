package io.github.kimzam30.nhakobeam.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf
import io.github.kimzam30.nhakobeam.design.NhakoSound

/** Plays one of the app's sounds from a composable (App provides AppState.sound). Silent in previews and tests. */
val LocalSound = staticCompositionLocalOf<(NhakoSound) -> Unit> { {} }

/** A click handler that plays the tap first (for rows and links; buttons do this themselves). */
@Composable
fun withTap(onClick: () -> Unit): () -> Unit {
    val play = LocalSound.current
    return {
        play(NhakoSound.Tap)
        onClick()
    }
}
