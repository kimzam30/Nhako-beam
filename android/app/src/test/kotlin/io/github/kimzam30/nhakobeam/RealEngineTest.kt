package io.github.kimzam30.nhakobeam

import io.github.kimzam30.nhakobeam.core.NhakoException
import io.github.kimzam30.nhakobeam.core.EventListener
import io.github.kimzam30.nhakobeam.core.NhakoEngine
import io.github.kimzam30.nhakobeam.model.Connection
import io.github.kimzam30.nhakobeam.model.ContractJson
import io.github.kimzam30.nhakobeam.model.EngineEvent
import io.github.kimzam30.nhakobeam.model.PairingCode
import io.github.kimzam30.nhakobeam.model.SelfInfo
import io.github.kimzam30.nhakobeam.model.Snapshot
import io.github.kimzam30.nhakobeam.ui.Words
import io.github.kimzam30.nhakobeam.ui.components.wordsFromCodeUri
import java.io.File
import java.nio.file.Files
import java.util.Collections
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.Test

/**
 * The real Rust engine through the generated bindings, on this computer (the build makes a host
 * copy of libnhako_ffi for the bindings). Checks that what the engine sends decodes into the
 * Kotlin contract, so the two sides can't drift apart unnoticed.
 */
class RealEngineTest {
    private val dir = Files.createTempDirectory("nhako-engine").toFile()
    private val events = Collections.synchronizedList(mutableListOf<String>())
    private lateinit var engine: NhakoEngine

    @Before fun setUp() {
        val lib = System.getProperty("jna.library.path")?.let { File(it, "libnhako_ffi.so") }
        assumeTrue("host engine library not built", lib?.exists() == true)
        engine = NhakoEngine(File(dir, "config").path, File(dir, "inbox").path, object : EventListener {
            override fun onEvent(json: String) {
                events += json
            }
        })
        // As in the F-Droid build: no update check, so the test never reaches GitHub.
        engine.setFromStore(true)
    }

    @After fun tearDown() {
        if (::engine.isInitialized) {
            engine.stop()
            engine.close()
        }
        dir.deleteRecursively()
    }

    @Test fun speaksTheSameContract() {
        val snapshot = ContractJson.decodeFromString<Snapshot>(engine.start("Test phone"))
        assertTrue(snapshot.transfers.isEmpty())

        val me = ContractJson.decodeFromString<SelfInfo>(engine.getSelf())
        assertEquals("Test phone", me.name)
        assertEquals(File(dir, "inbox").path, me.saveDir)

        engine.renameSelf("Kitchen tablet")
        assertEquals("Kitchen tablet", ContractJson.decodeFromString<SelfInfo>(engine.getSelf()).name)

        val code = ContractJson.decodeFromString<PairingCode>(engine.createCode())
        assertEquals(4, code.words.size)
        assertTrue(code.words.all(Words::isWord))
        assertEquals(code.words, wordsFromCodeUri(code.uri))
        engine.cancelCode()

        assertEquals(Connection.Public, ContractJson.decodeFromString<Connection>(engine.getConnection()))
        val own = Connection.Own(relay = "http://192.168.1.20:3340", token = "t0k", lookup = null)
        engine.setConnection(ContractJson.encodeToString(Connection.serializer(), own))
        assertEquals(own, ContractJson.decodeFromString<Connection>(engine.getConnection()))
        engine.setConnection(ContractJson.encodeToString(Connection.serializer(), Connection.Public))

        // Phase 8: nearby search on and off (the engine restarts), History, the wake token.
        assertTrue(engine.getLanDiscovery())
        engine.setLanDiscovery(false)
        assertEquals(false, engine.getLanDiscovery())
        engine.setLanDiscovery(true)

        // Phase 11: who can send to me, saved and read back as the contract's names.
        assertEquals("yourDevices", engine.getWhoCanSend())
        engine.setWhoCanSend("everyoneNearby")
        assertEquals("everyoneNearby", engine.getWhoCanSend())
        engine.setWhoCanSend("yourDevices")
        assertTrue(ContractJson.decodeFromString<Snapshot>(engine.snapshot()).history.isEmpty())
        engine.clearHistory()
        engine.setWakeToken("dQw4w9WgXcQ:APA91bHn-test_token-0123456789abcdefghijklmnopqrstuvwxyz")
        engine.setWakeToken(null)

        // Phase 14: text and trust asks to a device that isn't there say so in plain words; the
        // snapshot carries the asks waiting here.
        val noSuch = "not-a-device"
        val textErr = runCatching { engine.sendText(noSuch, "hi") }.exceptionOrNull()
        assertTrue("sending text to nobody fails: $textErr", textErr is NhakoException.Failed)
        assertTrue(runCatching { engine.askTrust(noSuch) }.isFailure)
        engine.cancelTrust(noSuch)
        engine.answerTrust(noSuch, false)
        assertTrue(ContractJson.decodeFromString<Snapshot>(engine.snapshot()).trustAsks.isEmpty())

        // Phase 15: Share via link, with text only (no files to open). Off a local network it says
        // so in plain words; on one, the share decodes and Stop ends it.
        val shared = runCatching { engine.shareLink(emptyList(), "Wi-Fi password is on the fridge") }
        shared.onSuccess { json ->
            val share = ContractJson.decodeFromString<io.github.kimzam30.nhakobeam.model.LinkShare>(json)
            assertTrue(share.url, share.url.startsWith("http://"))
            assertEquals("Wi-Fi password is on the fridge", share.text)
            assertEquals(share.id, ContractJson.decodeFromString<Snapshot>(engine.snapshot()).linkShare?.id)
            engine.answerVisitor("nobody", true)
            engine.stopLink()
        }.onFailure { e ->
            assertTrue("off a network: $e", e is NhakoException.Failed && e.reason.startsWith("Connect to Wi-Fi"))
        }
        assertEquals(null, ContractJson.decodeFromString<Snapshot>(engine.snapshot()).linkShare)
        // A build from a store has no update check, and no setting for it.
        assertEquals(null, engine.getUpdateCheck())
        assertEquals(null, ContractJson.decodeFromString<Snapshot>(engine.snapshot()).update)

        // Everything the engine said along the way is something the app understands.
        Thread.sleep(300)
        val said = synchronized(events) { events.toList() }
        assertTrue("the engine sent events", said.isNotEmpty())
        for (json in said) {
            runCatching { ContractJson.decodeFromString<EngineEvent>(json) }
                .onFailure { throw AssertionError("couldn't read event: $json", it) }
        }
    }
}
