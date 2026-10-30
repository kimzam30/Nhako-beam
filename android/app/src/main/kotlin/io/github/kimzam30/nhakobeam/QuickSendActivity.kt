package io.github.kimzam30.nhakobeam

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.core.content.IntentCompat
import androidx.core.content.pm.ShortcutManagerCompat
import androidx.lifecycle.lifecycleScope
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.state.Sheet
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.LocalSound
import io.github.kimzam30.nhakobeam.ui.components.ToastHost
import io.github.kimzam30.nhakobeam.ui.screens.SheetHost
import io.github.kimzam30.nhakobeam.ui.theme.NhakoTheme
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull

/**
 * "Share" from another app, without opening Nhako Beam: the Send sheet slides up over that app,
 * and picking a device sends and closes it. A device picked in Android's share sheet (Direct
 * Share, platform/DirectShare.kt) gets the files at once, with only a short "Sending to" message.
 *
 * Android lets this screen read the shared files only while it's open, so it closes once the
 * engine has opened them ([io.github.kimzam30.nhakobeam.state.AppState.sending] back at 0). Anything
 * that needs the whole app (a code, a link, trust) hands the share to [MainActivity].
 */
class QuickSendActivity : ComponentActivity() {
    private val nhako get() = application as NhakoApplication
    private var leaving = false

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val app = nhako.app
        // Not set up yet: the app asks for a name first, then shows the files.
        if (!app.onboarded) return handOff()
        val target = intent.getStringExtra(ShortcutManagerCompat.EXTRA_SHORTCUT_ID)
        // Nothing shared: a device picked from the app icon's long-press menu opens the app.
        if (savedInstanceState == null && !share(target)) return if (target != null) handOff() else finish()
        setContent {
            NhakoTheme(nhako.prefs.theme, nhako.prefs.motion) {
                CompositionLocalProvider(LocalSound provides app.sound) {
                    Box(Modifier.fillMaxSize()) {
                        SheetHost(app)
                        ToastHost(app, Modifier.align(Alignment.BottomCenter).windowInsetsPadding(WindowInsets.navigationBars).padding(bottom = 136.dp))
                    }
                }
            }
        }
        lifecycleScope.launch { follow() }
    }

    /** Stages what was shared. Straight to [target] when one was picked in the share sheet. */
    private fun share(target: String?): Boolean {
        val app = nhako.app
        val text = intent.takeIf { it.action == Intent.ACTION_SEND }?.getStringExtra(Intent.EXTRA_TEXT)
        if (text != null && IntentCompat.getParcelableExtra(intent, Intent.EXTRA_STREAM, Uri::class.java) == null) {
            lifecycleScope.launch {
                if (target != null && awaitEngine()) {
                    sendingTo(target)
                    app.sendTextTo(target, text)
                } else app.shareText(text)
            }
            return true
        }
        val uris = sharedUris()
        if (uris.isEmpty()) return false
        lifecycleScope.launch {
            val picked = runCatching { app.engine.stage(uris.map { it.toString() }) }.getOrNull()
            if (picked == null || picked.files.isEmpty()) {
                Toast.makeText(this@QuickSendActivity, t("send.failed"), Toast.LENGTH_SHORT).show()
                return@launch finish()
            }
            if (picked.skipped > 0) {
                val skipped = if (picked.skipped == 1) t("send.skipped.one") else t("send.skipped.many", "n" to picked.skipped)
                Toast.makeText(this@QuickSendActivity, skipped, Toast.LENGTH_SHORT).show()
            }
            if (target != null && awaitEngine()) {
                sendingTo(target)
                app.sendTo(target, picked.files)
            } else {
                app.stage(picked.files)
                app.sheet = Sheet.Send
            }
        }
        return true
    }

    private fun sharedUris(): List<Uri> = when (intent.action) {
        Intent.ACTION_SEND -> listOfNotNull(IntentCompat.getParcelableExtra(intent, Intent.EXTRA_STREAM, Uri::class.java))
        Intent.ACTION_SEND_MULTIPLE -> IntentCompat.getParcelableArrayListExtra(intent, Intent.EXTRA_STREAM, Uri::class.java).orEmpty()
        else -> emptyList()
    }.ifEmpty {
        val clip = intent.clipData ?: return emptyList()
        (0 until clip.itemCount).mapNotNull { clip.getItemAt(it).uri }
    }

    /** After a cold start the engine takes a moment; a device picked in the share sheet waits for it. */
    private suspend fun awaitEngine(): Boolean =
        nhako.app.started || withTimeoutOrNull(8_000) { snapshotFlow { nhako.app.started }.first { it } } == true

    private fun sendingTo(deviceId: String) {
        Toast.makeText(this, t("quick.sending", "device" to nhako.app.deviceName(deviceId)), Toast.LENGTH_SHORT).show()
    }

    /**
     * Closes once nothing is left to do here: the sheet was closed or a device picked, and the
     * engine has the files open. A send that failed puts its files back and opens the sheet again.
     * A sheet for something else (a code, a link) goes to the whole app.
     */
    private suspend fun follow() {
        val app = nhako.app
        // Wait for the sheet or a send to begin (staging takes a moment).
        snapshotFlow { app.sheet != null || app.sending > 0 }.first { it }
        while (!leaving) {
            // Until the sheet closes (a device was picked, or it was closed by hand) or turns into
            // something else.
            val sheet = snapshotFlow { app.sheet }.first { it == null || (it != Sheet.Send && it !is Sheet.Compose) }
            if (sheet != null) return handOff()
            // Picking a device takes what's waiting and starts a send in the same moment.
            val sent = app.sending > 0 || !app.hasStaged
            snapshotFlow { app.sending }.first { it == 0 }
            if (sent && app.hasStaged) {
                // The send failed and put the files back (its message is showing): try again.
                app.sheet = Sheet.Send
                continue
            }
            // Closed without sending: nothing waits in the app's tray for files it can't open.
            if (app.hasStaged) app.clearStaged()
            leaving = true
            // The sheet's exit plays first.
            delay(NhakoMotion.Duration.slow.toLong())
            finish()
        }
    }

    /** The whole app takes the share: it stages it again under its own permission to read it. */
    private fun handOff() {
        leaving = true
        nhako.app.clearStaged()
        startActivity(Intent(intent).setClass(this, MainActivity::class.java).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION))
        finish()
    }

    override fun finish() {
        super.finish()
        // No slide: the sheet already moved.
        @Suppress("DEPRECATION")
        overridePendingTransition(0, 0)
    }
}
