package io.github.kimzam30.nhakobeam

import io.github.kimzam30.nhakobeam.engine.doneWithFiles
import io.github.kimzam30.nhakobeam.model.Side
import io.github.kimzam30.nhakobeam.model.Transfer
import io.github.kimzam30.nhakobeam.model.TransferDirection
import io.github.kimzam30.nhakobeam.model.TransferStatus
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** Try again reads through the same file handles, so they stay open while it can still be pressed. */
class SentFilesTest {
    private fun sent(status: TransferStatus, by: Side? = null) =
        Transfer(id = "t", direction = TransferDirection.Send, deviceId = "d", files = emptyList(), totalBytes = 1, status = status, cancelledBy = by)

    @Test fun keepsFilesWhileTryAgainCanStillAsk() {
        assertFalse(sent(TransferStatus.Declined).doneWithFiles())
        assertFalse(sent(TransferStatus.Cancelled, Side.Peer).doneWithFiles())
        assertFalse(sent(TransferStatus.Failed).doneWithFiles())
    }

    @Test fun letsGoWhenNothingCanAskAgain() {
        assertTrue(sent(TransferStatus.Done).doneWithFiles())
        assertTrue(sent(TransferStatus.Cancelled, Side.Me).doneWithFiles())
    }
}
