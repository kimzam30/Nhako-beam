package io.github.kimzam30.nhakobeam

import io.github.kimzam30.nhakobeam.model.Connection
import io.github.kimzam30.nhakobeam.ui.OwnForm
import io.github.kimzam30.nhakobeam.ui.Parsed
import io.github.kimzam30.nhakobeam.ui.ServerProblemCode
import io.github.kimzam30.nhakobeam.ui.hostOf
import io.github.kimzam30.nhakobeam.ui.parseLookup
import io.github.kimzam30.nhakobeam.ui.parseRelay
import io.github.kimzam30.nhakobeam.ui.parseToken
import io.github.kimzam30.nhakobeam.ui.readOwnForm
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/** The same cases as desktop/src/lib/engine/server.test.ts and core/src/server.rs. */
class ServerFormTest {
    private fun problem(p: Parsed<*>) = (p as? Parsed.Bad)?.problem

    @Test fun readsRelayAddressesLikeTheEngine() {
        assertEquals(Parsed.Ok("http://192.168.1.20:3340"), parseRelay(" http://192.168.1.20:3340 "))
        assertEquals(Parsed.Ok("https://relay.example.com/"), parseRelay("https://relay.example.com/"))
        assertEquals(ServerProblemCode.Empty, problem(parseRelay("")))
        assertEquals(ServerProblemCode.Scheme, problem(parseRelay("192.168.1.20:3340")))
        assertEquals(ServerProblemCode.Scheme, problem(parseRelay("ftp://x")))
        assertEquals(ServerProblemCode.Path, problem(parseRelay("http://x/relay")))
        assertEquals(ServerProblemCode.Extra, problem(parseRelay("http://x/?token=1")))
        assertEquals(ServerProblemCode.Extra, problem(parseRelay("http://user:pw@x/")))
    }

    @Test fun readsLookupServers() {
        assertEquals(Parsed.Ok("http://10.0.0.2:3341"), parseLookup("http://10.0.0.2:3341"))
        assertEquals(Parsed.Ok("https://dns.example.com/pkarr"), parseLookup("https://dns.example.com/pkarr"))
        assertEquals(ServerProblemCode.Scheme, problem(parseLookup("dns.example.com")))
    }

    @Test fun checksTokensLikeTheEngine() {
        assertEquals(Parsed.Ok(null), parseToken("  "))
        assertEquals(Parsed.Ok("ab-12_x"), parseToken(" ab-12_x "))
        assertEquals(ServerProblemCode.TokenChars, problem(parseToken("two words")))
        assertEquals(ServerProblemCode.TokenChars, problem(parseToken("café")))
        assertEquals(ServerProblemCode.TokenLong, problem(parseToken("a".repeat(513))))
    }

    @Test fun turnsTheFormIntoAConnection() {
        assertEquals(
            Connection.Own(relay = "http://10.0.0.2:3340", token = "t0k", lookup = null),
            readOwnForm(OwnForm(relay = "http://10.0.0.2:3340", token = " t0k ", lookup = "")).connection,
        )
        val bad = readOwnForm(OwnForm(relay = "10.0.0.2", token = "a b", lookup = "x"))
        assertNull(bad.connection)
        assertEquals(ServerProblemCode.Scheme, bad.relay)
        assertEquals(ServerProblemCode.TokenChars, bad.token)
        assertEquals(ServerProblemCode.Scheme, bad.lookup)
        assertEquals("192.168.1.20", hostOf("http://192.168.1.20:3340"))
    }
}
