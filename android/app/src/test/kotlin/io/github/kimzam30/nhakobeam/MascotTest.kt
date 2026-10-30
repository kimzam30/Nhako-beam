package io.github.kimzam30.nhakobeam

import androidx.test.core.app.ApplicationProvider
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.stage.MascotAssets
import io.github.kimzam30.nhakobeam.stage.MascotMotion
import io.github.kimzam30.nhakobeam.stage.MascotState
import io.github.kimzam30.nhakobeam.stage.MoodInput
import io.github.kimzam30.nhakobeam.stage.StageCamera
import io.github.kimzam30.nhakobeam.stage.mascotMood
import io.github.kimzam30.nhakobeam.stage.nextChange
import io.github.kimzam30.nhakobeam.state.Moments
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** The voxel mascot (Phase 22): the model loads, the camera and the mood rules match desktop's. */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36])
class MascotTest {
    private val model = MascotAssets.get(ApplicationProvider.getApplicationContext())!!

    @Test fun theModelLoadsWithEveryPart() {
        val names = model.parts.map { it.name }
        for (n in listOf("body", "side", "eyeL", "eyeR", "mouth", "cheeks", "sparkle1", "eyeXL", "eyeHappyL", "mouthOpen", "crack", "zzz")) assertTrue(n, n in names)
        assertTrue(model.part("crack")!!.hidden)
        assertEquals(1 / 16f, model.rootScale, 1e-6f)
        assertEquals(10, model.data.states.size)
        assertEquals(4, model.data.devices.size)
    }

    // The same as desktop/src/lib/stage/stage.test.ts.
    @Test fun theCameraPutsItsTargetInTheMiddleAndMirrors() {
        val cam = StageCamera.home(model.data.stage)
        val p = cam.project(cam.target[0], cam.target[1], cam.target[2], 400f, 340f)
        assertEquals(200f, p.x, 1e-3f)
        assertEquals(170f, p.y, 1e-3f)
        val l = cam.project(-2f, 0f, 0f, 400f, 340f)
        val r = cam.project(2f, 0f, 0f, 400f, 340f)
        assertEquals(400f, l.x + r.x, 1e-3f)
    }

    private val base = MoodInput(100_000, started = true, offline = false, searching = false, devices = 2, sending = false, receiving = false, looking = false, moments = Moments())

    @Test fun theMoodFollowsTheSameRulesAsDesktop() {
        val cases = listOf(
            base.copy(started = false) to MascotState.Loading,
            base.copy(offline = true) to MascotState.Offline,
            base to MascotState.Idle,
            base.copy(devices = 0) to MascotState.Asleep,
            base.copy(devices = 0, searching = true) to MascotState.Looking,
            base.copy(sending = true) to MascotState.Sending,
            base.copy(receiving = true) to MascotState.Receiving,
            base.copy(moments = Moments(found = 99_500)) to MascotState.Happy,
            base.copy(sending = true, moments = Moments(done = 99_000)) to MascotState.Done,
            base.copy(moments = Moments(done = 99_900, failed = 99_000)) to MascotState.Failed,
            base.copy(moments = Moments(done = 100_000L - NhakoMotion.Mascot.doneMs - 1)) to MascotState.Idle,
            base.copy(offline = true, moments = Moments(done = 99_900)) to MascotState.Offline,
            base.copy(looking = true) to MascotState.Looking,
        )
        for ((input, want) in cases) assertEquals(input.toString(), want, mascotMood(input))
        assertNull(nextChange(base))
        assertEquals(99_000L + NhakoMotion.Mascot.doneMs, nextChange(base.copy(moments = Moments(done = 99_000))))
    }

    @Test fun stillPosesAndTheDoneBurst() {
        val m = MascotMotion(model.data) { 0.5f }
        m.set(MascotState.Failed, 0)
        val failed = m.still()
        assertEquals("x", failed.face)
        assertTrue(failed.crack)
        m.set(MascotState.Asleep, 0)
        assertNotNull(m.still().zzzRise)
        m.set(MascotState.Done, 1000)
        var bursts = 0
        var top = 0f
        var t = 1000L
        while (t < 4000) {
            val p = m.step(t, 0.016f, null)
            if (p.burst) bursts++
            top = maxOf(top, p.y)
            t += 16
        }
        assertEquals(1, bursts)
        assertTrue(top > 0.7f)
    }
}
