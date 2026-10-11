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
import androidx.core.view.WindowCompat
import io.github.kimzam30.nhakobeam.platform.ActivityPlatform
import io.github.kimzam30.nhakobeam.state.PlatformHooks
import io.github.kimzam30.nhakobeam.state.ThemePref
import io.github.kimzam30.nhakobeam.ui.NhakoBeamApp

class MainActivity : ComponentActivity() {
    private val nhako get() = application as NhakoApplication
    private lateinit var platform: ActivityPlatform

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
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
            NhakoBeamApp(app, prefs)
        }
        if (savedInstanceState == null) receiveShared(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        receiveShared(intent)
    }

    override fun onDestroy() {
        if (nhako.app.platform === platform) nhako.app.platform = object : PlatformHooks {}
        super.onDestroy()
    }

    /** Files shared from another app ("Share" then Nhako Beam): stage them and pick a device. */
    private fun receiveShared(intent: Intent?) {
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
