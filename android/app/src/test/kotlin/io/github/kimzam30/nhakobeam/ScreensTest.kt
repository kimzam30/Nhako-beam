package io.github.kimzam30.nhakobeam

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
import io.github.kimzam30.nhakobeam.model.TransferDirection
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.MemoryPrefs
import io.github.kimzam30.nhakobeam.state.Screen
import io.github.kimzam30.nhakobeam.state.Sheet
import io.github.kimzam30.nhakobeam.state.ThemePref
import io.github.kimzam30.nhakobeam.ui.NhakoBeamApp
import org.junit.Rule
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

    @Test fun serverBannerAndOffline() {
        show {
            connection = Connection.Own("http://192.168.1.20:3340", "secret")
            onEvent(EngineEvent.Server(ServerProblem.TokenRejected))
            onEvent(EngineEvent.Discovery(DiscoveryState.Offline))
        }
        shot("12-banners")
    }
}
