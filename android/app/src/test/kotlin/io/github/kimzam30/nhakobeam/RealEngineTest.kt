package io.github.kimzam30.nhakobeam

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
        assertEquals(3, code.words.size)
        assertTrue(code.words.all(Words::isWord))
        assertEquals(code.words, wordsFromCodeUri(code.uri))
        engine.cancelCode()

        assertEquals(Connection.Public, ContractJson.decodeFromString<Connection>(engine.getConnection()))
        val own = Connection.Own(relay = "http://192.168.1.20:3340", token = "t0k", lookup = null)
        engine.setConnection(ContractJson.encodeToString(Connection.serializer(), own))
        assertEquals(own, ContractJson.decodeFromString<Connection>(engine.getConnection()))
        engine.setConnection(ContractJson.encodeToString(Connection.serializer(), Connection.Public))

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
