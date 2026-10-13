package io.github.kimzam30.nhakobeam.engine

import io.github.kimzam30.nhakobeam.model.Connection
import io.github.kimzam30.nhakobeam.model.EngineEvent
import io.github.kimzam30.nhakobeam.model.FileItem
import io.github.kimzam30.nhakobeam.model.PairingCode
import io.github.kimzam30.nhakobeam.model.RedeemResult
import io.github.kimzam30.nhakobeam.model.SelfInfo
import io.github.kimzam30.nhakobeam.model.ServerCheck
import io.github.kimzam30.nhakobeam.model.Snapshot
import io.github.kimzam30.nhakobeam.model.WhoCanSend
import kotlinx.coroutines.flow.Flow

/**
 * The app's view of the engine: the same contract as the desktop app's (types.ts), in Kotlin.
 * - [RealEngine]: the Rust core (ffi/), for the app.
 * - [FakeEngine]: a pretend one, for previews and screenshot tests.
 * Calls that answer are `suspend`; the rest return at once and report back through [events].
 */
interface Engine {
    val events: Flow<EngineEvent>

    /** Starts the engine (once) and returns what to draw. [name] is saved first (onboarding). */
    suspend fun start(name: String? = null): Snapshot
    fun stop()
    fun rescan()

    /** Whether the app is on screen. In the background the engine checks on devices less often. */
    fun setForeground(on: Boolean)

    /** Turns files picked or shared from other apps (content:// URIs) into items to send. */
    suspend fun stage(uris: List<String>): Staged
    /** Turns a folder picked in the system picker (a tree content:// URI) into items to send,
     *  each with the folder it's in. */
    suspend fun stageFolder(treeUri: String): Staged
    /** Resolves to the transfer id. Throws [EngineException] with a message ready to show. */
    suspend fun send(deviceId: String, files: List<FileItem>): String
    /** Sends text (a note, a link, a clipboard). Resolves to the transfer id. */
    suspend fun sendText(deviceId: String, text: String): String
    suspend fun accept(requestId: String, trust: Boolean = false, autoAccept: Boolean = false): String?
    fun decline(requestId: String)
    fun pause(transferId: String)
    fun resume(transferId: String)
    fun cancel(transferId: String)
    fun retry(transferId: String)

    suspend fun createCode(): PairingCode
    fun cancelCode()
    suspend fun redeemCode(words: List<String>): RedeemResult

    fun setTrusted(deviceId: String, trusted: Boolean)
    /** Asks a nearby device to trust this one, and to be trusted back. [EngineEvent.TrustWaiting]
     *  follows, then [EngineEvent.Paired] (via nearby) or [EngineEvent.TrustEnded]. */
    suspend fun askTrust(deviceId: String)
    fun cancelTrust(deviceId: String)
    /** Answers another device's ask. */
    fun answerTrust(deviceId: String, yes: Boolean)
    fun setAutoAccept(deviceId: String, on: Boolean)
    suspend fun renameSelf(name: String)
    suspend fun getSelf(): SelfInfo

    suspend fun getConnection(): Connection
    /** Saves the setting and reconnects through it. Throws [EngineException] if it can't. */
    suspend fun setConnection(c: Connection)
    /** Never throws: problems come back in the result. */
    suspend fun checkServer(c: Connection.Own): ServerCheck

    /** Where a received transfer's files are, to open or share them (also from History). */
    suspend fun savedFiles(transferId: String): List<String>

    /** Whether this device announces itself and looks for others on the local network. */
    suspend fun getLanDiscovery(): Boolean
    /** Saves it and reconnects. Throws [EngineException] if it can't (transfers still running). */
    suspend fun setLanDiscovery(on: Boolean)

    /** Who may send requests here. Applies to the next request, no reconnect. */
    suspend fun getWhoCanSend(): WhoCanSend
    suspend fun setWhoCanSend(who: WhoCanSend)

    /** Forgets every History entry. Received files stay where they are. */
    fun clearHistory()

    /** Settings > About > Copy diagnostics: a plain-text report on the network and nearby
     *  discovery, with recent logs. Leaves out device names, file names and internet addresses. */
    suspend fun diagnostics(): String

    /** This phone's wake token from Firebase (null when push isn't set up), shared with trusted devices. */
    fun setWakeToken(token: String?) {}
    /** Whether other devices can wake this one: a wake service was built in. */
    val wakeAvailable: Boolean get() = false
}

/** Files ready to send, and how many shared items couldn't be read. */
data class Staged(val files: List<FileItem>, val skipped: Int)

/** A problem from the engine, in plain language, ready to show. */
class EngineException(message: String) : Exception(message)
