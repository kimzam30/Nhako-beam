package io.github.kimzam30.nhakobeam

import android.app.NotificationManager
import android.content.Context
import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.performClick
import androidx.test.core.app.ApplicationProvider
import io.github.kimzam30.nhakobeam.design.NhakoSound
import io.github.kimzam30.nhakobeam.engine.FakeEngine
import io.github.kimzam30.nhakobeam.model.EngineEvent
import io.github.kimzam30.nhakobeam.model.IncomingRequest
import io.github.kimzam30.nhakobeam.model.PairVia
import io.github.kimzam30.nhakobeam.model.TransferDirection
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.platform.Notifier
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.MemoryPrefs
import io.github.kimzam30.nhakobeam.state.Screen
import io.github.kimzam30.nhakobeam.state.Sheet
import io.github.kimzam30.nhakobeam.state.SoundPlayer
import io.github.kimzam30.nhakobeam.state.ToastTone
import io.github.kimzam30.nhakobeam.ui.NhakoBeamApp
import io.github.kimzam30.nhakobeam.ui.Copy.t
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

/** The sounds: what decides whether one plays, what the app plays when, and the notification channels. */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36])
class SoundsTest {
    @get:Rule val compose = createComposeRule()

    private val heard = mutableListOf<NhakoSound>()
    private val queued = mutableListOf<Runnable>()
    private var clock = 0L
    private var on = true
    private val player = SoundPlayer({ heard += it }, { on }, { queued += it }, { queued -= it }, { clock })

    private fun runQueued() = queued.toList().also { queued.clear() }.forEach { it.run() }

    @Test fun aTapWaitsAndAnotherSoundReplacesIt() {
        player.play(NhakoSound.Tap)
        player.play(NhakoSound.Tap)
        runQueued()
        assertEquals(listOf(NhakoSound.Tap), heard)

        player.play(NhakoSound.Tap)
        player.play(NhakoSound.Accept)
        runQueued()
        assertEquals(listOf(NhakoSound.Tap, NhakoSound.Accept), heard)
    }

    @Test fun theSameSoundTwiceInARowPlaysOnce() {
        player.play(NhakoSound.Accept)
        clock += 380
        player.play(NhakoSound.Accept)
        clock += 500
        player.play(NhakoSound.Accept)
        assertEquals(listOf(NhakoSound.Accept, NhakoSound.Accept), heard)
    }

    @Test fun quietWhenOffIncludingThePressThatTurnsItOff() {
        player.play(NhakoSound.Tap)
        on = false
        runQueued()
        player.play(NhakoSound.Done)
        assertTrue(heard.isEmpty())
    }

    @Test fun theAppPlaysTheSameSoundsAsTheDesktopApp() {
        val fake = FakeEngine()
        val app = AppState(fake, MemoryPrefs(), CoroutineScope(Dispatchers.Unconfined))
        app.sound = { heard += it }

        app.onEvent(EngineEvent.Incoming(IncomingRequest("r1", "d-aisyah", fake.photos(), 100, 0)))
        val receiving = fake.transfer(TransferDirection.Receive, "d-aisyah", fake.photos(), TransferStatus.Transferring)
        app.onEvent(EngineEvent.TransferChanged(receiving))
        app.onEvent(EngineEvent.TransferChanged(receiving.copy(status = TransferStatus.Done)))
        app.onEvent(EngineEvent.TransferChanged(fake.transfer(TransferDirection.Send, "d-aisyah", fake.photos(), TransferStatus.Failed)))
        app.sheet = Sheet.Send
        app.sheet = null
        app.onEvent(EngineEvent.Paired(fake.devices.first(), PairVia.Entered, "AB12 CD34"))
        app.navigate(Screen.Settings)
        app.navigate(Screen.Home)
        app.toast("No space left", ToastTone.Danger)

        assertEquals(
            listOf(
                NhakoSound.Incoming, NhakoSound.Done, NhakoSound.Failed, NhakoSound.Open, NhakoSound.Close,
                NhakoSound.Matched, NhakoSound.Open, NhakoSound.Close, NhakoSound.Error,
            ),
            heard,
        )
    }

    @Test fun pressingSettingsPlaysOpenNotATapToo() {
        val handler = Handler(Looper.getMainLooper())
        val live = SoundPlayer({ heard += it }, { true }, { handler.post(it) }, { handler.removeCallbacks(it) })
        val prefs = MemoryPrefs()
        compose.setContent {
            val scope = rememberCoroutineScope()
            val app = remember { AppState(FakeEngine(), prefs, scope).also { it.sound = live::play; it.init() } }
            NhakoBeamApp(app, prefs)
        }
        compose.mainClock.advanceTimeBy(1500)
        heard.clear()
        compose.onNodeWithContentDescription(t("settings.title")).performClick()
        shadowOf(Looper.getMainLooper()).idle()
        compose.waitForIdle()
        assertEquals(listOf(NhakoSound.Open), heard)
    }

    @Test fun pauseAndCancelOnATransferPlayTheirOwnSoundsAsOnDesktop() {
        val handler = Handler(Looper.getMainLooper())
        val live = SoundPlayer({ heard += it }, { true }, { handler.post(it) }, { handler.removeCallbacks(it) })
        val prefs = MemoryPrefs()
        val fake = FakeEngine()
        lateinit var app: AppState
        compose.setContent {
            val scope = rememberCoroutineScope()
            app = remember { AppState(fake, prefs, scope).also { it.sound = live::play; it.init() } }
            NhakoBeamApp(app, prefs)
        }
        compose.mainClock.advanceTimeBy(1500)
        compose.runOnIdle { app.onEvent(EngineEvent.TransferChanged(fake.transfer(TransferDirection.Receive, "d-aisyah", fake.photos(), TransferStatus.Transferring, 0.4))) }
        compose.mainClock.advanceTimeBy(1500)
        heard.clear()
        compose.onNodeWithContentDescription(t("transfer.pause")).performClick()
        compose.mainClock.advanceTimeBy(600)
        shadowOf(Looper.getMainLooper()).idle()
        compose.onNodeWithContentDescription(t("send.cancel")).performClick()
        compose.mainClock.advanceTimeBy(600)
        shadowOf(Looper.getMainLooper()).idle()
        compose.waitForIdle()
        assertEquals(listOf(NhakoSound.Pause, NhakoSound.Cancel), heard)
    }

    @Test fun notificationsUseTheNewSounds() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        Notifier(context)
        val nm = context.getSystemService(NotificationManager::class.java)
        val requests = nm.getNotificationChannel(Notifier.CHANNEL_REQUESTS)
        val received = nm.getNotificationChannel(Notifier.CHANNEL_RECEIVED)
        assertEquals("android.resource://${context.packageName}/${NhakoSound.Incoming.res}", requests.sound.toString())
        assertEquals("android.resource://${context.packageName}/${NhakoSound.Done.res}", received.sound.toString())
        assertEquals(null, nm.getNotificationChannel("requests"))
    }
}
