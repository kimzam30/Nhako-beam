package io.github.kimzam30.nhakobeam

import io.github.kimzam30.nhakobeam.model.Transfer
import io.github.kimzam30.nhakobeam.model.TransferDirection
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.state.ActivityItem
import io.github.kimzam30.nhakobeam.state.activityItems
import io.github.kimzam30.nhakobeam.state.groupProgress
import org.junit.Assert.assertEquals
import org.junit.Test

/** The same cases as the desktop's groups.test.ts. */
class SendGroupsTest {
    private fun tr(id: String, done: Long = 0, total: Long = 100, status: TransferStatus = TransferStatus.Transferring) =
        Transfer(id = id, direction = TransferDirection.Send, deviceId = "d-$id", files = emptyList(), totalBytes = total, doneBytes = done, status = status)

    @Test fun foldsAGroupIntoOneCardWhereItsFirstTransferWas() {
        val items = activityItems(listOf(tr("a"), tr("b"), tr("c"), tr("d")), mapOf("g1" to listOf("b", "d")))
        assertEquals(listOf("a", "g1:bd", "c"), items.map { if (it is ActivityItem.Group) "${it.id}:${it.transfers.joinToString("") { x -> x.id }}" else it.key })
    }

    @Test fun aGroupWithOneTransferLeftShownIsThatTransferAgain() {
        assertEquals(listOf(ActivityItem.One(tr("a"))), activityItems(listOf(tr("a")), mapOf("g1" to listOf("a", "b"))))
    }

    @Test fun countsEveryByteSoAFastDeviceDoesNotHideASlowOne() {
        assertEquals(0.5f, groupProgress(listOf(tr("a", done = 100, status = TransferStatus.Done), tr("b"))))
        assertEquals(1f, groupProgress(listOf(tr("a", total = 0, status = TransferStatus.Done), tr("b", total = 0, status = TransferStatus.Done))))
    }
}
