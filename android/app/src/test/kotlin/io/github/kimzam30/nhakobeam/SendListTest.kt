package io.github.kimzam30.nhakobeam

import io.github.kimzam30.nhakobeam.model.Device
import io.github.kimzam30.nhakobeam.model.DeviceKind
import io.github.kimzam30.nhakobeam.model.Route
import io.github.kimzam30.nhakobeam.state.canTrust
import io.github.kimzam30.nhakobeam.state.sendGroups
import io.github.kimzam30.nhakobeam.state.withoutTwins
import io.github.kimzam30.nhakobeam.model.OsPlatform
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SendListTest {
    private fun dev(id: String, name: String, trusted: Boolean = false, reachable: Boolean = true, route: Route? = Route.Nearby, nearby: Boolean = true) =
        Device(id, name, DeviceKind.Phone, trusted = trusted, reachable = reachable, route = route, nearby = nearby)

    @Test fun keepsRowsInNameOrderWhateverOrderTheEngineListsThem() {
        val a = dev("a", "Kims S24"); val b = dev("b", "kims flip"); val c = dev("c", "Kims Tab")
        assertEquals(listOf("b", "a", "c"), sendGroups(listOf(a, b, c)).second.map { it.id })
        assertEquals(listOf("b", "a", "c"), sendGroups(listOf(c, a, b)).second.map { it.id })
    }

    @Test fun aDeviceOfYoursDoesNotMoveWhenItGoesOfflineOrChangesRoute() {
        val a = dev("a", "Alpha", trusted = true); val b = dev("b", "Beta", trusted = true)
        assertEquals(listOf("a", "b"), sendGroups(listOf(b, a)).first.map { it.id })
        assertEquals(listOf("a", "b"), sendGroups(listOf(a.copy(reachable = false, route = null), b.copy(route = Route.Direct))).first.map { it.id })
    }

    @Test fun aReinstalledDeviceShowsOnceItsOfflineTwinHidesWhileItIsOnline() {
        val old = dev("old", "Tab", trusted = true, reachable = false, route = null); val fresh = dev("new", "Tab", trusted = true)
        assertEquals(listOf("new"), sendGroups(listOf(old, fresh)).first.map { it.id })
        assertEquals(listOf("old", "new"), withoutTwins(listOf(old, fresh.copy(reachable = false))).map { it.id })
        for (o in listOf(old.copy(name = "Tab 2"), old.copy(kind = DeviceKind.Tablet), old.copy(os = OsPlatform.Windows))) assertEquals(2, withoutTwins(listOf(o, fresh)).size)
        assertEquals(2, withoutTwins(listOf(old, fresh.copy(trusted = false))).size)
    }

    @Test fun trustGoesByBeingOnThisNetworkNotByTheRoute() {
        assertTrue(canTrust(dev("a", "A", route = Route.Direct)))
        assertFalse(canTrust(dev("a", "A", route = Route.Relayed, nearby = false)))
        assertFalse(canTrust(dev("a", "A", trusted = true)))
        assertFalse(canTrust(dev("a", "A", reachable = false)))
    }
}
