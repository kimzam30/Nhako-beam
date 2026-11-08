package io.github.kimzam30.nhakobeam

import io.github.kimzam30.nhakobeam.engine.FakeEngine
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.MemoryPrefs
import io.github.kimzam30.nhakobeam.state.Sheet
import io.github.kimzam30.nhakobeam.state.ToastTone
import io.github.kimzam30.nhakobeam.ui.Copy.t
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** Scanning a pairing code's QR. The paired event swaps the Scan sheet for the Paired one before
 *  the engine's answer arrives; a pairing that worked must never end on "That code didn't match". */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36])
class PairingTest {
    private fun app(fake: FakeEngine) = AppState(fake, MemoryPrefs(), CoroutineScope(Dispatchers.Unconfined)).also {
        it.sound = {}
        it.init()
        it.sheet = Sheet.Scan
    }

    @Test fun aReinstalledDeviceCanReplaceItsOldEntryOrSitBesideIt() {
        val fake = FakeEngine()
        fake.devices = fake.devices.map { if (it.id == "d-office") it.copy(autoAccept = true) else it }
        val app = app(fake).also { it.sheet = null }
        val fresh = fake.simReinstall("d-office")
        val ask = app.sheet as Sheet.TrustAsk
        assertEquals("d-office", ask.replaces)
        app.answerTrust(fresh, true, replace = true)
        assertTrue(app.devices.none { it.id == "d-office" })
        assertTrue(app.devices.single { it.id == fresh }.let { it.trusted && it.autoAccept })

        // Keep both: the old one stays.
        val fake2 = FakeEngine()
        val app2 = app(fake2).also { it.sheet = null }
        val fresh2 = fake2.simReinstall("d-office")
        app2.answerTrust(fresh2, true, replace = false)
        assertTrue(app2.devices.any { it.id == "d-office" && it.trusted })
        assertTrue(app2.devices.single { it.id == fresh2 }.trusted)
    }

    @Test fun aScannedCodeThatWorksShowsOnlyThePairedSheet() {
        val fake = FakeEngine()
        val app = app(fake)
        var done = false
        app.redeemScanned(fake.peerCode) { done = true }
        assertTrue(done)
        assertTrue("sheet was ${app.sheet}", app.sheet is Sheet.Paired)
        assertTrue(app.toasts.none { it.tone == ToastTone.Danger })
    }

    @Test fun aScannedCodeThatDoesntMatchSaysSoAndOffersTyping() {
        val app = app(FakeEngine())
        app.redeemScanned(listOf("ember", "marigold", "meadow", "willow"))
        assertEquals(Sheet.EnterCode, app.sheet)
        assertEquals(listOf(t("code.enter.wrong")), app.toasts.filter { it.tone == ToastTone.Danger }.map { it.text })
    }
}
