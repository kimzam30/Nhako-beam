package io.github.kimzam30.nhakobeam

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.remember
import androidx.compose.ui.unit.dp
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.SemanticsMatcher
import com.github.takahirom.roborazzi.RobolectricDeviceQualifiers
import com.github.takahirom.roborazzi.captureRoboImage
import io.github.kimzam30.nhakobeam.engine.FakeEngine
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
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/**
 * Accessibility (Phase 15): on the main screens and sheets, everything you can tap has a name a
 * screen reader can say and a touch area of at least 48 dp, and the screens still fit with the
 * system font at 200%. Colors are checked once, from the tokens (desktop: src/lib/contrast.test.ts).
 * The desktop twin is desktop/tests/e2e/a11y.spec.ts.
 *
 * Google's Accessibility Test Framework doesn't run under Robolectric (it passes everything), so
 * these checks read the Compose semantics tree themselves; [checksCatchABadButton] proves they bite.
 */
@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = RobolectricDeviceQualifiers.Pixel7)
class AccessibilityTest {
    @get:Rule val compose = createComposeRule()

    private lateinit var app: AppState
    private val fake = FakeEngine()
    private val prefs = MemoryPrefs(onboarded = true)

    private fun show(setUp: AppState.() -> Unit = {}) {
        compose.setContent {
            val scope = rememberCoroutineScope()
            app = remember { AppState(fake, prefs, scope).also { it.init() } }
            NhakoBeamApp(app, prefs, false)
        }
        compose.mainClock.advanceTimeBy(1200)
        compose.runOnIdle { app.setUp() }
        compose.mainClock.advanceTimeBy(1600)
    }

    /** Fails with every tappable thing that has no name, or is smaller than 48 by 48 dp. */
    private fun check() {
        val tappable = compose.onAllNodes(SemanticsMatcher.keyIsDefined(SemanticsActions.OnClick), useUnmergedTree = false).fetchSemanticsNodes()
        val density = RuntimeEnvironment.getApplication().resources.displayMetrics.density
        val problems = tappable.mapNotNull { node ->
            val c = node.config
            if (c.getOrNull(SemanticsProperties.Disabled) != null) return@mapNotNull null
            // Scrolled out of view: checked on the screens where it shows.
            if (node.boundsInRoot.width <= 0f || node.boundsInRoot.height <= 0f) return@mapNotNull null
            val name = listOfNotNull(
                c.getOrNull(SemanticsProperties.ContentDescription)?.joinToString(" "),
                c.getOrNull(SemanticsProperties.Text)?.joinToString(" ") { it.text },
                c.getOrNull(SemanticsProperties.EditableText)?.text?.takeIf { c.getOrNull(SemanticsProperties.ContentDescription) == null }?.let { "" },
                c.getOrNull(SemanticsProperties.StateDescription),
                c.getOrNull(SemanticsActions.OnClick)?.label,
            ).joinToString(" ").trim()
            val touch = node.touchBoundsInRoot
            val w = touch.width / density
            val h = touch.height / density
            val where = "at ${touch.left.toInt()},${touch.top.toInt()} (${"%.0f".format(w)} x ${"%.0f".format(h)} dp)"
            when {
                name.isEmpty() -> "no name $where"
                w < 47.5f || h < 47.5f -> "\"$name\" too small $where"
                else -> null
            }
        }
        org.junit.Assert.assertTrue("accessibility problems:\n" + problems.joinToString("\n"), problems.isEmpty())
    }

    @Test fun home() {
        show()
        check()
    }

    @Test fun homeDark() {
        prefs.theme = ThemePref.Dark
        show()
        check()
    }

    @Test fun sendSheetAndLink() {
        show {
            stage(fake.photos())
            sheet = Sheet.Send
        }
        check()
        compose.runOnIdle { app.shareLink() }
        compose.mainClock.advanceTimeBy(1600)
        compose.runOnIdle { fake.simVisitor() }
        compose.mainClock.advanceTimeBy(1200)
        check()
    }

    @Test fun request() {
        show { fake.simIncoming() }
        check()
    }

    @Test fun settings() {
        show { navigate(Screen.Settings) }
        check()
    }

    @Test fun history() {
        show { navigate(Screen.History) }
        check()
    }

    @Test fun showCode() {
        show { sheet = Sheet.ShowCode }
        check()
    }

    /** The system font at its largest (200%): screenshots to look at, and the checks still pass. */
    @Test fun largestFont() {
        RuntimeEnvironment.setFontScale(2f)
        show { stage(fake.photos()) }
        compose.onRoot().captureRoboImage("build/screenshots/37-font-200-home.png")
        compose.runOnIdle { app.sheet = Sheet.Send }
        compose.mainClock.advanceTimeBy(1200)
        compose.onRoot().captureRoboImage("build/screenshots/38-font-200-send.png")
        compose.runOnIdle {
            app.sheet = null
            fake.simIncoming()
        }
        compose.mainClock.advanceTimeBy(1600)
        compose.onRoot().captureRoboImage("build/screenshots/39-font-200-request.png")
        compose.runOnIdle {
            app.decline(app.requests.first().id)
            app.navigate(Screen.Settings)
        }
        compose.mainClock.advanceTimeBy(1600)
        compose.onRoot().captureRoboImage("build/screenshots/40-font-200-settings.png")
        check()
    }

    /** The checks really run here: a tiny button with no name is caught. */
    @Test fun checksCatchABadButton() {
        compose.setContent {
            androidx.compose.foundation.layout.Box(
                androidx.compose.ui.Modifier.size(8.dp).clickable {},
            )
        }
        val caught = runCatching { check() }.exceptionOrNull()
        org.junit.Assert.assertTrue("a bad button passed the checks", caught != null)
    }
}
