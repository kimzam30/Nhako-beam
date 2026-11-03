package io.github.kimzam30.nhakobeam

import androidx.test.core.app.ApplicationProvider
import android.content.Context
import io.github.kimzam30.nhakobeam.engine.FakeEngine
import io.github.kimzam30.nhakobeam.model.Device
import io.github.kimzam30.nhakobeam.model.DeviceKind
import io.github.kimzam30.nhakobeam.model.OsPlatform
import io.github.kimzam30.nhakobeam.stage.RoomData
import io.github.kimzam30.nhakobeam.stage.arrangeRoom
import io.github.kimzam30.nhakobeam.stage.fitRoom
import io.github.kimzam30.nhakobeam.stage.roomSlots
import io.github.kimzam30.nhakobeam.stage.roomTags
import io.github.kimzam30.nhakobeam.stage.roomView
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.MemoryPrefs
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** Which device stands where. The same cases as desktop/src/lib/stage/room.test.ts. */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36])
class RoomTest {
    private val data = RoomData.parse(ApplicationProvider.getApplicationContext<Context>().assets.open("room.json").use { it.reader().readText() })
    private fun dev(id: String, kind: DeviceKind) = Device(id, id, kind, OsPlatform.Linux, trusted = true, reachable = true)
    private val mine = listOf(dev("a-phone", DeviceKind.Phone), dev("b-laptop", DeviceKind.Laptop), dev("c-pc", DeviceKind.Desktop), dev("d-pc", DeviceKind.Desktop), dev("e-phone", DeviceKind.Phone), dev("f-tablet", DeviceKind.Tablet))

    private val flip = 360f to 880f
    private val screens = listOf(flip, 384f to 832f, 823f to 1317f, 1317f to 823f, 884f to 1104f, 880f to 360f, 300f to 288f, 320f to 640f, 900f to 640f, 1280f to 800f, 1920f to 1080f, 2560f to 1080f)
    private val kinds = listOf(DeviceKind.Desktop, DeviceKind.Laptop, DeviceKind.Tablet, DeviceKind.Phone)
    private fun arrange(list: List<Device>, picks: Set<String>? = null, screen: Pair<Float, Float> = flip) = arrangeRoom(list, picks, data, screen.first, screen.second)

    @Test fun showsAllFourOfFourOnAPhone() {
        val r = arrange(mine.take(4))
        assertEquals(4, r.room.size)
        assertTrue(r.rest.isEmpty())
    }

    @Test fun givesPcsTheDeskFirst() {
        assertEquals(listOf("deskL", "deskR"), arrange(mine.take(4)).room.filter { it.device.kind == DeviceKind.Desktop }.map { it.slot }.sorted())
    }

    @Test fun keepsToYourPicks() {
        val r = arrange(mine, setOf("a-phone", "c-pc"))
        assertEquals(setOf("a-phone", "c-pc"), r.room.map { it.device.id }.toSet())
        assertEquals(4, r.rest.size)
    }

    @Test fun aThirdPcOnAPhoneGoesInTheRest() {
        val r = arrange(listOf(dev("p1", DeviceKind.Desktop), dev("p2", DeviceKind.Desktop), dev("p3", DeviceKind.Desktop)))
        assertEquals(2, r.room.size)
        assertEquals(listOf("p3"), r.rest.map { it.id })
    }

    @Test fun theCoverScreenKeepsTheRoomAPicture() {
        val r = arrange(mine, screen = 300f to 288f)
        assertTrue(r.room.isEmpty())
        assertEquals(mine.size, r.rest.size)
    }

    @Test fun everyMixOnEveryScreenStaysOnItsPlaceAndOnScreen() {
        for ((w, h) in screens) {
            val f = fitRoom(data, w, h)
            val offered = roomSlots(data, w, h).associateBy { it.id }
            for (a in 0..8) for (b in 0..8 - a) for (c in 0..8 - a - b) for (d in 0..8 - a - b - c) {
                val devices = listOf(a, b, c, d).flatMapIndexed { k, n -> List(n) { i -> dev("${kinds[k]}$i", kinds[k]) } }
                val r = arrangeRoom(devices, null, data, w, h)
                assertEquals(devices.size, r.room.size + r.rest.size)
                assertEquals(r.room.size, r.room.map { it.slot }.toSet().size)
                for (q in r.room) {
                    assertTrue("${q.device.kind} on ${q.slot}, ${w}x$h", offered.getValue(q.slot).kinds.containsKey(q.device.kind.name.lowercase()))
                    assertTrue(q.at.margin >= 0f)
                    assertTrue("${q.slot}, ${w}x$h", f.ox + q.at.box.x0 * f.k >= 0f && f.ox + q.at.box.x1 * f.k <= w)
                }
                val tags = roomTags(data, r.room, f, w)
                for (t in tags) for (u in tags) if (t !== u) assertFalse("${t.id} touches ${u.id}, ${w}x$h", kotlin.math.abs(t.x - u.x) < (t.w + u.w) / 2 && t.y < u.y + data.tag.h && u.y < t.y + data.tag.h)
            }
        }
    }

    @Test fun everyPlaceHoldsWhatItOffers() {
        for (s in data.slots) for ((kind, q) in s.kinds) assertTrue("$kind on ${s.id}", q.margin >= 0f)
    }

    @Test fun aCropForEachShapeOfScreen() {
        assertEquals("tall", roomView(data, 360f, 880f))
        assertEquals("mid", roomView(data, 823f, 1317f))
        assertEquals("mid", roomView(data, 884f, 1104f))
        assertEquals("wide", roomView(data, 1317f, 823f))
    }

    @Test fun theFitNeverShowsPastTheArtsEdges() {
        for ((w, h) in screens) {
            val f = fitRoom(data, w, h)
            val v = data.views.getValue(f.view)
            assertTrue(f.ox + v.x * f.k <= 0.01f && f.oy + v.y * f.k <= 0.01f)
            assertTrue(f.ox + (v.x + v.w) * f.k >= w - 0.01f && f.oy + (v.y + v.h) * f.k >= h - 0.01f)
        }
    }

    @Test fun unticksStayOutAndAFullRoomRefuses() {
        val fake = FakeEngine()
        val app = AppState(fake, MemoryPrefs(), CoroutineScope(Dispatchers.Unconfined))
        val (w, h) = flip
        app.devices = mine + dev("g-phone", DeviceKind.Phone)
        // Seven devices, five places: untick one, and it stays out.
        assertTrue(app.toggleRoom("a-phone", data, w, h))
        assertFalse(app.roomLayout(data, w, h).room.any { it.device.id == "a-phone" })
        assertEquals(4, app.roomLayout(data, w, h).room.size)
        assertTrue(app.toggleRoom("a-phone", data, w, h))
        assertFalse(app.toggleRoom("g-phone", data, w, h))
    }
}
