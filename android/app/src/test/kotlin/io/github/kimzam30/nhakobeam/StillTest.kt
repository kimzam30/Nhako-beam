package io.github.kimzam30.nhakobeam

import io.github.kimzam30.nhakobeam.engine.FakeEngine
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.MemoryPrefs
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** The room holds still after a quiet while. The engine's routine checks don't count as news. */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36])
class StillTest {
    private val quiet = 300L

    private fun waitFor(what: String, ok: () -> Boolean) {
        val until = System.currentTimeMillis() + 5_000
        while (!ok()) {
            check(System.currentTimeMillis() < until) { "timed out waiting for $what" }
            Thread.sleep(10)
        }
    }

    @Test fun routineDeviceUpdatesDontWakeItButANewDeviceDoes() {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        val fake = FakeEngine()
        val app = AppState(fake, MemoryPrefs(), scope, stillAfterMs = quiet).also { it.sound = {}; it.init() }
        waitFor("devices") { app.devices.isNotEmpty() }
        waitFor("still") { app.still }
        // Route changes every 50 ms for longer than the quiet while: still stays still.
        repeat(12) { fake.simRouteChange("d-aisyah"); Thread.sleep(50) }
        assertTrue("a route change woke it", app.still)
        fake.simNewDevice()
        waitFor("woken by a new device") { !app.still }
        waitFor("still again") { app.still }
        app.poke()
        assertFalse(app.still)
        scope.cancel()
    }
}
