package io.github.kimzam30.nhakobeam.ui

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.Prefs
import io.github.kimzam30.nhakobeam.state.Screen
import io.github.kimzam30.nhakobeam.ui.components.IncomingLayer
import io.github.kimzam30.nhakobeam.ui.components.ToastHost
import io.github.kimzam30.nhakobeam.ui.screens.HomeScreen
import io.github.kimzam30.nhakobeam.ui.screens.OnboardingScreen
import io.github.kimzam30.nhakobeam.ui.screens.SettingsScreen
import io.github.kimzam30.nhakobeam.ui.screens.SheetHost
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.NhakoTheme

/** The whole app: the current screen, and the layers on top (incoming request, sheets, toasts). */
@Composable
fun NhakoBeamApp(app: AppState, prefs: Prefs) {
    NhakoTheme(prefs.theme, prefs.motion) {
        val reduced = Nhako.reducedMotion
        Box(Modifier.fillMaxSize().background(Nhako.colors.canvas)) {
            // Forward slides in from the right on the gentle spring while the old screen shifts a
            // little the other way and fades; back is the mirror image.
            AnimatedContent(
                app.screen,
                transitionSpec = {
                    val d = app.navDir
                    if (reduced) fadeIn(tween(150)) togetherWith fadeOut(tween(120))
                    else (slideInHorizontally(NhakoMotion.gentle.spec()) { d * it / 16 } + fadeIn(tween(NhakoMotion.Duration.base))) togetherWith
                        (slideOutHorizontally(tween(NhakoMotion.Duration.fast)) { -d * it / 48 } + fadeOut(tween(NhakoMotion.Duration.fast)))
                },
                label = "screen",
            ) { screen ->
                when (screen) {
                    Screen.Onboarding -> OnboardingScreen(app)
                    Screen.Home -> HomeScreen(app)
                    Screen.Settings -> SettingsScreen(app, prefs)
                }
            }
            IncomingLayer(app)
            SheetHost(app)
            ToastHost(app, Modifier.align(Alignment.BottomCenter).windowInsetsPadding(WindowInsets.navigationBars).padding(bottom = 136.dp))
        }
    }
}
