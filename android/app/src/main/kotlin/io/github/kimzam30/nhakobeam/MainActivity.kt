package io.github.kimzam30.nhakobeam

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.LaunchedEffect
import androidx.core.content.IntentCompat
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.core.view.WindowCompat
import io.github.kimzam30.nhakobeam.platform.ActivityPlatform
import io.github.kimzam30.nhakobeam.state.PlatformHooks
import io.github.kimzam30.nhakobeam.state.ThemePref
import io.github.kimzam30.nhakobeam.ui.NhakoBeamApp

class MainActivity : ComponentActivity() {
    private val nhako get() = application as NhakoApplication
    private lateinit var platform: ActivityPlatform

    override fun onCreate(savedInstanceState: Bundle?) {
        // The system splash (Theme.Nhako.Splash) shows until the first frame; the Compose intro
        // draws the same picture, so it's removed at once rather than faded.
        installSplashScreen().setOnExitAnimationListener { it.remove() }
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        // Only a fresh launch from the launcher plays the intro. Shared files go straight to the
        // device picker, and a screen rotation or coming back from the background shows nothing.
        val intro = savedInstanceState == null && intent?.action == Intent.ACTION_MAIN
        val app = nhako.app
        val prefs = nhako.prefs
        platform = ActivityPlatform(this, app)
        app.platform = platform
        setContent {
            // The status and navigation bar icons follow the app's theme, not only the system's.
            val dark = when (prefs.theme) {
                ThemePref.System -> isSystemInDarkTheme()
                ThemePref.Dark -> true
                ThemePref.Light -> false
            }
            LaunchedEffect(dark) {
                WindowCompat.getInsetsController(window, window.decorView).apply {
                    isAppearanceLightStatusBars = !dark
                    isAppearanceLightNavigationBars = !dark
                }
            }
            NhakoBeamApp(app, prefs, intro)
        }
        if (savedInstanceState == null) receiveShared(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        receiveShared(intent)
    }

    // On screen or not: the engine checks on your devices less often in the background.
    override fun onStart() {
        super.onStart()
        nhako.app.engine.setForeground(true)
    }

    override fun onStop() {
        nhako.app.engine.setForeground(false)
        super.onStop()
    }

    override fun onDestroy() {
        if (nhako.app.platform === platform) nhako.app.platform = object : PlatformHooks {}
        super.onDestroy()
    }

    /** Files shared from another app, handed over by [QuickSendActivity] (before setup is done, or
     *  for a code, a link or trust): stage them and pick a device. A link or text shared from the
     *  browser or a notes app goes as text. */
    private fun receiveShared(intent: Intent?) {
        val text = intent?.takeIf { it.action == Intent.ACTION_SEND }?.getStringExtra(Intent.EXTRA_TEXT)
        if (text != null && IntentCompat.getParcelableExtra(intent, Intent.EXTRA_STREAM, Uri::class.java) == null) {
            nhako.app.shareText(text)
            return
        }
        val uris: List<Uri> = when (intent?.action) {
            Intent.ACTION_SEND -> listOfNotNull(IntentCompat.getParcelableExtra(intent, Intent.EXTRA_STREAM, Uri::class.java))
            Intent.ACTION_SEND_MULTIPLE -> IntentCompat.getParcelableArrayListExtra(intent, Intent.EXTRA_STREAM, Uri::class.java).orEmpty()
            else -> return
        }.ifEmpty {
            val clip = intent.clipData ?: return
            (0 until clip.itemCount).mapNotNull { clip.getItemAt(it).uri }
        }
        nhako.app.stageUris(uris.map { it.toString() }, openSendSheet = true)
    }
}
