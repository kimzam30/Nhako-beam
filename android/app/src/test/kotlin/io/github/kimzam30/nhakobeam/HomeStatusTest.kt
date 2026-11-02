package io.github.kimzam30.nhakobeam

import io.github.kimzam30.nhakobeam.engine.FakeEngine
import io.github.kimzam30.nhakobeam.model.DiscoveryState
import io.github.kimzam30.nhakobeam.model.Transfer
import io.github.kimzam30.nhakobeam.model.TransferDirection
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.state.StatusTone
import io.github.kimzam30.nhakobeam.state.homeStatus
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Test

/** Home's heading. The same cases as desktop/src/lib/state/status.test.ts. */
class HomeStatusTest {
    private val fake = FakeEngine()
    private fun moving(direction: TransferDirection = TransferDirection.Send, status: TransferStatus = TransferStatus.Transferring, done: Double = 0.25) =
        fake.transfer(direction, "d-studio", fake.photos(), status, done)

    private fun status(
        discovery: DiscoveryState = DiscoveryState.Idle,
        active: List<Transfer> = emptyList(),
        reachable: Int = 0,
        staging: Boolean = false,
    ) = homeStatus(discovery, active, reachable, staging) { "Kim's XPS" }

    @Test fun saysReadyAndCountsDevicesOnceFound() {
        assertEquals("Ready to beam", status().text)
        assertEquals("1 device ready", status(reachable = 1).text)
        assertEquals("3 devices ready", status(reachable = 3).text)
    }

    @Test fun saysItIsLookingWhileItLooks() {
        val s = status(DiscoveryState.Searching)
        assertEquals("Looking for devices", s.text)
        assertEquals(StatusTone.Searching, s.tone)
    }

    @Test fun countsWhatItFoundEvenWhileItKeepsLooking() {
        val s = status(DiscoveryState.Searching, reachable = 5)
        assertEquals("5 devices ready", s.text)
        assertEquals(StatusTone.Ready, s.tone)
    }

    @Test fun putsATransferBeforeAnythingButBeingOffline() {
        val sending = status(DiscoveryState.Searching, listOf(moving()), staging = true)
        assertEquals("Sending to Kim's XPS", sending.text)
        assertEquals(25, sending.percent)
        assertEquals("Receiving from Kim's XPS", status(active = listOf(moving(TransferDirection.Receive))).text)
        assertEquals("Waiting for Kim's XPS", status(active = listOf(moving(status = TransferStatus.Waiting))).text)
        assertEquals("Connecting to Kim's XPS", status(active = listOf(moving(status = TransferStatus.Reconnecting))).text)
        assertEquals(StatusTone.Warn, status(active = listOf(moving(status = TransferStatus.Paused))).tone)
        assertEquals("2 transfers going", status(active = listOf(moving(), moving())).text)
        assertEquals("You're offline", status(DiscoveryState.Offline, listOf(moving())).text)
    }

    @Test fun keepsTheSameKeyWhileOnlyThePercentMoves() {
        val t = moving(done = 0.1)
        assertEquals(status(active = listOf(t)).key, status(active = listOf(t.copy(doneBytes = t.totalBytes - 1))).key)
        assertNotEquals(status(active = listOf(t.copy(status = TransferStatus.Waiting))).key, status(active = listOf(t)).key)
    }

    @Test fun asksForADeviceWhenFilesAreWaiting() {
        assertEquals("Pick a device", status(DiscoveryState.Searching, staging = true).text)
    }
}
