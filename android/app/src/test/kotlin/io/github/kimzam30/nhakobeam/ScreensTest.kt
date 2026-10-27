package io.github.kimzam30.nhakobeam

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.ui.components.Bolt
import io.github.kimzam30.nhakobeam.ui.components.BoltMode
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.NhakoTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onRoot
import io.github.kimzam30.nhakobeam.ui.screens.readLicenses
import org.junit.Assert.assertTrue
import com.github.takahirom.roborazzi.RobolectricDeviceQualifiers
import com.github.takahirom.roborazzi.captureRoboImage
import io.github.kimzam30.nhakobeam.engine.FakeEngine
import io.github.kimzam30.nhakobeam.model.Connection
import io.github.kimzam30.nhakobeam.model.Device
import io.github.kimzam30.nhakobeam.model.DeviceKind
import io.github.kimzam30.nhakobeam.model.OsPlatform
import io.github.kimzam30.nhakobeam.model.DiscoveryState
import io.github.kimzam30.nhakobeam.model.EngineEvent
import io.github.kimzam30.nhakobeam.model.PairVia
import io.github.kimzam30.nhakobeam.model.ServerProblem
import io.github.kimzam30.nhakobeam.model.FileItem
import io.github.kimzam30.nhakobeam.model.TransferDirection
import io.github.kimzam30.nhakobeam.model.TransferError
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.MemoryPrefs
import io.github.kimzam30.nhakobeam.state.Screen
import io.github.kimzam30.nhakobeam.state.Sheet
import io.github.kimzam30.nhakobeam.state.ThemePref
import io.github.kimzam30.nhakobeam.ui.NhakoBeamApp
import org.junit.Rule
import org.junit.Assert.assertEquals
import androidx.compose.ui.test.onNodeWithText
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/**
 * Renders every main state of the Android app on the JVM (Robolectric) with the pretend engine and
 * saves a screenshot of each to build/screenshots/. Run: ./gradlew testDebugUnitTest
 */
@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = RobolectricDeviceQualifiers.Pixel7)
class ScreensTest {
    @get:Rule val compose = createComposeRule()

    private lateinit var app: AppState
    private val fake = FakeEngine()
    private val prefs = MemoryPrefs(onboarded = true)

    /** Shows the app; [setUp] runs once the engine has started (devices are in). */
    private fun show(setUp: AppState.() -> Unit = {}) {
        compose.setContent { Host() }
        compose.mainClock.advanceTimeBy(1200)
        compose.runOnIdle { app.setUp() }
        compose.mainClock.advanceTimeBy(1600)
    }

    @Composable
    private fun Host(intro: Boolean = false) {
        val scope = rememberCoroutineScope()
        app = remember { AppState(fake, prefs, scope).also { it.init() } }
        NhakoBeamApp(app, prefs, intro)
    }

    /** Phase 8: the launch intro, caught mid-way (the bolt pressed in, the ring going out). */
    private fun showIntro(at: Long) {
        compose.mainClock.autoAdvance = false
        compose.setContent { Host(intro = true) }
        compose.mainClock.advanceTimeBy(at)
    }

    private fun shot(name: String) {
        compose.onRoot().captureRoboImage("build/screenshots/$name.png")
    }

    @Test fun onboardingWelcome() {
        prefs.onboarded = false
        show()
        shot("01-onboarding-welcome")
    }

    @Test fun home() {
        show()
        shot("02-home")
    }

    @Test fun homeDark() {
        prefs.theme = ThemePref.Dark
        show()
        shot("03-home-dark")
    }

    @Test fun stagedAndSendSheet() {
        show { stage(fake.photos()) }
        shot("04-home-staged")
        compose.runOnIdle { app.sheet = Sheet.Send }
        compose.mainClock.advanceTimeBy(1200)
        shot("05-send-sheet")
    }

    @Test fun incomingRequest() {
        show { fake.simIncoming() }
        shot("06-incoming")
    }

    @Test fun receiving() {
        show {
            onEvent(EngineEvent.TransferChanged(fake.transfer(TransferDirection.Receive, "d-aisyah", fake.mixed(), TransferStatus.Transferring, 0.42)))
            onEvent(EngineEvent.TransferChanged(fake.transfer(TransferDirection.Send, "d-studio", fake.photos(), TransferStatus.Done, 1.0)))
        }
        shot("07-receiving")
    }

    @Test fun showCode() {
        show { sheet = Sheet.ShowCode }
        compose.mainClock.advanceTimeBy(1500)
        shot("08-show-code")
    }

    @Test fun enterCode() {
        show { sheet = Sheet.EnterCode }
        shot("09-enter-code")
    }

    @Test fun paired() {
        show { onEvent(EngineEvent.Paired(fake.devices[0], PairVia.Entered, "5D19 E04A")) }
        compose.mainClock.advanceTimeBy(1200)
        shot("10-paired")
    }

    @Test fun settings() {
        show { navigate(Screen.Settings) }
        shot("11-settings")
    }

    @Test fun introPressed() {
        showIntro(150)
        shot("13-intro-pressed")
    }

    @Test fun introRing() {
        showIntro(420)
        shot("14-intro-ring")
    }

    @Test fun history() {
        show { navigate(Screen.History) }
        shot("15-history")
    }

    @Test fun historyDark() {
        prefs.theme = ThemePref.Dark
        show { navigate(Screen.History) }
        shot("16-history-dark")
    }

    @Test fun historyItem() {
        show {
            navigate(Screen.History)
            sheet = Sheet.HistoryItem("h1")
        }
        compose.mainClock.advanceTimeBy(1200)
        shot("17-history-item")
    }

    /** Phase 13: the bolt as an icon, caught mid-strike (sparks out) and settled. */
    @Test fun boltStates() {
        compose.mainClock.autoAdvance = false
        compose.setContent {
            NhakoTheme {
                val c = Nhako.colors
                Row(Modifier.background(c.canvas).padding(32.dp), horizontalArrangement = Arrangement.spacedBy(36.dp)) {
                    Bolt(56.dp)
                    Bolt(56.dp, mode = BoltMode.Check, tint = c.success, strike = true)
                    Bolt(56.dp, mode = BoltMode.Cracked, brand = false, tint = c.danger)
                    Bolt(56.dp, mode = BoltMode.Asleep)
                }
            }
        }
        compose.mainClock.advanceTimeBy(560)
        shot("25-bolt-strike")
        compose.mainClock.advanceTimeBy(2000)
        shot("26-bolt-states")
    }

    @Test fun historyEmpty() {
        fake.history = emptyList()
        show { navigate(Screen.History) }
        shot("18-history-empty")
    }

    @Test fun licenses() {
        // The list ships in the app and must read: Rust crates and the Android libraries, each with a text.
        val list = readLicenses(androidx.test.core.app.ApplicationProvider.getApplicationContext())
        assertTrue(list.packages.size > 100)
        assertTrue(list.packages.any { it.name == "iroh" } && list.packages.any { it.ecosystem == "android" })
        assertTrue(list.packages.all { p -> p.texts.isNotEmpty() && p.texts.all { it in list.texts } })
        show { navigate(Screen.Licenses) }
        // The list loads off the main thread; rows show "version · license" once it's in.
        compose.waitUntil(5000) { compose.onAllNodesWithText(" · ", substring = true).fetchSemanticsNodes().isNotEmpty() }
        compose.mainClock.advanceTimeBy(800)
        shot("19-licenses")
    }

    @Test fun asleepPhoneAndWaking() {
        fake.devices = fake.devices + Device("d-mum", "Mum's phone", DeviceKind.Phone, OsPlatform.Android, trusted = true, fingerprint = "71C0 9EA2", wakeable = true)
        show {
            onEvent(EngineEvent.TransferChanged(fake.transfer(TransferDirection.Send, "d-mum", fake.photos(), TransferStatus.Connecting).copy(waking = true)))
        }
        shot("20-asleep-and-waking")
    }

    /** Phase 11: a stranger's request is marked, a refused send says why, and a received program asks first. */
    @Test fun notYoursRequest() {
        show { fake.simIncoming("d-studio") }
        shot("21-not-yours-request")
    }

    @Test fun refusedBecauseNotTheirs() {
        show {
            onEvent(EngineEvent.TransferChanged(fake.transfer(TransferDirection.Send, "d-studio", fake.photos(), TransferStatus.Declined).copy(error = TransferError.NotAllowed)))
        }
        shot("22-not-allowed")
    }

    @Test fun receivedProgramAsksFirst() {
        val setup = fake.transfer(TransferDirection.Receive, "d-studio", listOf(FileItem("f0", "Setup.exe", 38_000_000, "application/x-msdownload")), TransferStatus.Done, 1.0)
        show {
            onEvent(EngineEvent.TransferChanged(setup))
            openReceived(setup)
        }
        compose.mainClock.advanceTimeBy(800)
        compose.runOnIdle { assertTrue(app.sheet is Sheet.ConfirmOpen) }
        shot("23-confirm-open")
        compose.runOnIdle {
            app.sheet = null
            app.openReceived(setup.copy(files = listOf(setup.files[0].copy(name = "Holiday.jpg"))))
        }
        compose.runOnIdle { assertTrue("a photo opens straight away", app.sheet == null) }
    }

    /** Phase 12: a folder of 1,240 photos lists its files a page at a time, and progress updates merge in. */
    @Test fun bigFolderFilesSheet() {
        val photos = List(1240) { i -> FileItem("f$i", "IMG_${4001 + i}.jpg", 2_000_000, "image/jpeg") }
        val folder = fake.transfer(TransferDirection.Receive, "d-aisyah", photos, TransferStatus.Transferring, 0.0)
        show {
            onEvent(EngineEvent.TransferChanged(folder))
            onEvent(EngineEvent.TransferProgress(folder.id, 5_000_000, 40_000_000.0, 1L, mapOf("f0" to 2_000_000L, "f1" to 2_000_000L, "f2" to 1_000_000L)))
            sheet = Sheet.TransferDetails(folder.id)
        }
        compose.mainClock.advanceTimeBy(1200)
        compose.runOnIdle {
            val t = app.transfers.first { it.id == folder.id }
            assertEquals(5_000_000L, t.doneBytes)
            assertEquals(1_000_000L, t.fileBytes["f2"])
            assertEquals(0L, t.fileBytes["f3"] ?: 0L)
        }
        compose.onNodeWithText("Show 100 more").assertExists()
        shot("24-big-folder")
    }

    @Test fun serverBannerAndOffline() {
        show {
            connection = Connection.Own("http://192.168.1.20:3340", "secret")
            onEvent(EngineEvent.Server(ServerProblem.TokenRejected))
            onEvent(EngineEvent.Discovery(DiscoveryState.Offline))
        }
        shot("12-banners")
    }

    /** Phase 14: text arrives in the request, to read before Copy or Open link. */
    @Test fun linkRequest() {
        show { fake.simIncomingText(text = FakeEngine.SAMPLE_LINK) }
        compose.onNodeWithText(FakeEngine.SAMPLE_LINK).assertExists()
        compose.onNodeWithText("Open link").assertExists()
        shot("27-link-request")
    }

    @Test fun folderRequestAndTextCard() {
        show {
            onEvent(EngineEvent.TransferChanged(fake.transfer(TransferDirection.Receive, "d-phone", listOf(fake.file("Text.txt", 60, "text/plain")), TransferStatus.Done, 1.0).copy(text = FakeEngine.SAMPLE_TEXT)))
            fake.simIncoming("d-aisyah", fake.tree())
        }
        compose.onNodeWithText("Langkawi trip").assertExists()
        shot("28-folder-request")
    }

    @Test fun composeText() {
        show { sheet = Sheet.Compose(text = "Meet at 6 by the north gate.\nBring the charger.") }
        shot("29-compose")
        compose.runOnIdle { assertTrue("too long is turned away", !app.stageText("x".repeat(64 * 1024 + 1))) }
        compose.runOnIdle {
            app.sheet = null
            app.shareText("https://nhako.example/trip/photos")
            assertEquals("https://nhako.example/trip/photos", app.stagedText)
            assertEquals(Sheet.Send, app.sheet)
        }
    }

    @Test fun deviceSheetAndTrust() {
        show { sheet = Sheet.DeviceSheet("d-studio") }
        compose.onNodeWithText("Trust this device").assertExists()
        shot("30-device-sheet")
        fake.nextTrustReply = null
        compose.runOnIdle { app.askTrust("d-studio") }
        compose.mainClock.advanceTimeBy(1600)
        compose.runOnIdle { assertEquals(Sheet.TrustWait("d-studio", FakeEngine.TRUST_CHECK), app.sheet) }
        shot("31-trust-wait")
        compose.runOnIdle {
            app.onEvent(EngineEvent.TrustEnded("d-studio", io.github.kimzam30.nhakobeam.model.TrustOutcome.Declined))
            // Checked at once: the test clock runs a toast's time out while waiting for idle.
            assertEquals(null, app.sheet)
            assertTrue("toasts: ${app.toasts.map { it.text }}", app.toasts.any { it.text == "Studio PC said no." })
        }
    }

    @Test fun beingAskedThenPaired() {
        show { fake.simTrustAsk("d-studio") }
        compose.mainClock.advanceTimeBy(800)
        compose.onNodeWithText(FakeEngine.TRUST_CHECK).assertExists()
        shot("32-trust-ask")
        compose.runOnIdle { app.answerTrust("d-studio", true) }
        compose.mainClock.advanceTimeBy(1800)
        compose.runOnIdle { assertTrue(app.sheet is Sheet.Paired) }
        compose.onNodeWithText("Studio PC is one of your devices now").assertExists()
        shot("33-paired-nearby")
    }

    @Test fun offWifi() {
        show {
            onLan = false
            devices = devices.map { it.copy(reachable = false, route = null) }
        }
        compose.onNodeWithText("You're not on Wi-Fi").assertExists()
        shot("34-off-wifi")
    }

    @Test fun shareViaLink() {
        show { stage(fake.photos()) }
        compose.runOnIdle { app.shareLink() }
        compose.mainClock.advanceTimeBy(1600)
        compose.runOnIdle {
            assertEquals(Sheet.Link, app.sheet)
            assertTrue("the tray empties", app.staged.isEmpty())
        }
        compose.onNodeWithText(FakeEngine.LINK_URL).assertExists()
        compose.onNodeWithText("Nobody has opened it yet.").assertExists()
        compose.runOnIdle {
            fake.simVisitor("Chrome on Android", "192.168.1.23")
            fake.simVisitor("Edge on Windows", "192.168.1.41", io.github.kimzam30.nhakobeam.model.VisitorState.Allowed, done = 0.42)
        }
        compose.mainClock.advanceTimeBy(1200)
        compose.onNodeWithText("192.168.1.23 · Wants to download").assertExists()
        compose.onNodeWithText("192.168.1.41 · Downloading, 42%").assertExists()
        shot("35-share-via-link")
        compose.runOnIdle { app.answerVisitor(app.linkShare!!.visitors[0].id, false) }
        compose.mainClock.advanceTimeBy(600)
        compose.onNodeWithText("192.168.1.23 · Turned away").assertExists()
        compose.runOnIdle { app.sheet = null }
        compose.mainClock.advanceTimeBy(1200)
        compose.onNodeWithText("Sharing a link").assertExists()
        compose.runOnIdle {
            app.stopLink()
            assertEquals(null, app.linkShare)
        }
    }

    @Test fun shareViaLinkNeedsWifi() {
        fake.onWifi = false
        show {
            onLan = false
            stage(fake.photos())
            sheet = Sheet.Send
        }
        compose.onNodeWithText("Connect to Wi-Fi to share with a link").assertExists()
    }

    @Test fun updateBanner() {
        show { fake.simUpdate("0.3.0") }
        compose.onNodeWithText("Nhako Beam 0.3.0 is out").assertExists()
        shot("36-update-banner")
        compose.runOnIdle { app.hideUpdate() }
        compose.mainClock.advanceTimeBy(600)
        compose.onNodeWithText("Nhako Beam 0.3.0 is out").assertDoesNotExist()
        compose.runOnIdle { assertEquals("0.3.0", prefs.updateSeen) }
    }
}
