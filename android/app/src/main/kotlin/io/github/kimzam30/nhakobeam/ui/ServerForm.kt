// Your own server settings: the same rules as desktop/src/lib/engine/server.ts and
// core/src/server.rs, so the form can say what's wrong before anything is saved.
package io.github.kimzam30.nhakobeam.ui

import io.github.kimzam30.nhakobeam.model.Connection
import java.net.URI

/** What's wrong with a field, as a copy key suffix (`server.problem.<code>`). */
enum class ServerProblemCode(val code: String) { Empty("empty"), Scheme("scheme"), NoHost("noHost"), Path("path"), Extra("extra"), TokenChars("tokenChars"), TokenLong("tokenLong") }

sealed interface Parsed<out T> {
    data class Ok<T>(val value: T) : Parsed<T>
    data class Bad(val problem: ServerProblemCode) : Parsed<Nothing>
}

private fun parseHttpUrl(raw: String): Parsed<URI> {
    val s = raw.trim()
    if (s.isEmpty()) return Parsed.Bad(ServerProblemCode.Empty)
    if (!s.contains("://")) return Parsed.Bad(ServerProblemCode.Scheme)
    val uri = runCatching { URI(s) }.getOrElse { return Parsed.Bad(ServerProblemCode.NoHost) }
    if (uri.scheme != "http" && uri.scheme != "https") return Parsed.Bad(ServerProblemCode.Scheme)
    if (uri.host.isNullOrEmpty()) return Parsed.Bad(ServerProblemCode.NoHost)
    if (uri.rawQuery != null || uri.rawFragment != null || uri.rawUserInfo != null || s.contains('?') || s.contains('#')) return Parsed.Bad(ServerProblemCode.Extra)
    return Parsed.Ok(uri)
}

/** A relay address is just the server: `http(s)://host[:port]`. */
fun parseRelay(raw: String): Parsed<String> = when (val r = parseHttpUrl(raw)) {
    is Parsed.Bad -> r
    is Parsed.Ok -> if (r.value.rawPath.isNullOrEmpty() || r.value.rawPath == "/") Parsed.Ok(raw.trim()) else Parsed.Bad(ServerProblemCode.Path)
}

/** The lookup server's address; a bare server is fine (the engine adds `/pkarr`). */
fun parseLookup(raw: String): Parsed<String> = when (val r = parseHttpUrl(raw)) {
    is Parsed.Bad -> r
    is Parsed.Ok -> Parsed.Ok(raw.trim())
}

/** Printable ASCII, no spaces, at most 512 characters. Empty means no token. */
fun parseToken(raw: String): Parsed<String?> {
    val s = raw.trim()
    if (s.isEmpty()) return Parsed.Ok(null)
    if (s.length > 512) return Parsed.Bad(ServerProblemCode.TokenLong)
    if (!s.all { it.code in 0x21..0x7e }) return Parsed.Bad(ServerProblemCode.TokenChars)
    return Parsed.Ok(s)
}

data class OwnForm(val relay: String = "", val token: String = "", val lookup: String = "")

data class FormRead(
    val relay: ServerProblemCode? = null,
    val token: ServerProblemCode? = null,
    val lookup: ServerProblemCode? = null,
    val connection: Connection.Own? = null,
)

/** Checks the form; returns the problems, or the connection to save. */
fun readOwnForm(f: OwnForm): FormRead {
    val relay = parseRelay(f.relay)
    val token = parseToken(f.token)
    val lookup = if (f.lookup.isBlank()) Parsed.Ok(null) else parseLookup(f.lookup)
    val read = FormRead(
        relay = (relay as? Parsed.Bad)?.problem,
        token = (token as? Parsed.Bad)?.problem,
        lookup = (lookup as? Parsed.Bad)?.problem,
    )
    if (relay !is Parsed.Ok || token !is Parsed.Ok || lookup !is Parsed.Ok) return read
    return read.copy(connection = Connection.Own(relay = f.relay.trim(), token = token.value, lookup = lookup.value))
}

/** "192.168.1.20" from "http://192.168.1.20:3340", for short status lines. */
fun hostOf(address: String): String = runCatching { URI(address.trim()).host }.getOrNull() ?: address
