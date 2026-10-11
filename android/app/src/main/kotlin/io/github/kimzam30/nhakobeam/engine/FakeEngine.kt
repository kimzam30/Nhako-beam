package io.github.kimzam30.nhakobeam.engine

import io.github.kimzam30.nhakobeam.model.CheckResult
import io.github.kimzam30.nhakobeam.model.Connection
import io.github.kimzam30.nhakobeam.model.Device
import io.github.kimzam30.nhakobeam.model.DeviceKind
import io.github.kimzam30.nhakobeam.model.DiscoveryState
import io.github.kimzam30.nhakobeam.model.EngineEvent
import io.github.kimzam30.nhakobeam.model.FileItem
import io.github.kimzam30.nhakobeam.model.IncomingRequest
import io.github.kimzam30.nhakobeam.model.OsPlatform
import io.github.kimzam30.nhakobeam.model.PairVia
import io.github.kimzam30.nhakobeam.model.PairingCode
import io.github.kimzam30.nhakobeam.model.RedeemResult
import io.github.kimzam30.nhakobeam.model.Route
import io.github.kimzam30.nhakobeam.model.SelfInfo
import io.github.kimzam30.nhakobeam.model.ServerCheck
import io.github.kimzam30.nhakobeam.model.ServerProblem
import io.github.kimzam30.nhakobeam.model.Snapshot
import io.github.kimzam30.nhakobeam.model.Transfer
import io.github.kimzam30.nhakobeam.model.TransferDirection
import io.github.kimzam30.nhakobeam.model.TransferStatus
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow

/**
 * A pretend engine with the desktop prototype's sample devices, for previews and the screenshot
 * tests. Nothing moves on its own: tests push states with the `sim*` calls, so screenshots are exact.
 */
class FakeEngine : Engine {
    private val _events = MutableSharedFlow<EngineEvent>(extraBufferCapacity = 256)
    override val events: SharedFlow<EngineEvent> = _events

    var devices = listOf(
        Device("d-aisyah", "Aisyah's laptop", DeviceKind.Laptop, OsPlatform.Windows, trusted = true, reachable = true, route = Route.Nearby, fingerprint = "4F2A 91C3"),
        Device("d-phone", "Work phone", DeviceKind.Phone, OsPlatform.Android, trusted = true, autoAccept = true, reachable = true, route = Route.Nearby, fingerprint = "B7E0 22D9"),
        Device("d-studio", "Studio PC", DeviceKind.Desktop, OsPlatform.Linux, reachable = true, route = Route.Nearby, fingerprint = "0C5D E81A"),
        Device("d-tablet", "Mei Ling's tablet", DeviceKind.Tablet, OsPlatform.Android, reachable = true, route = Route.Nearby, fingerprint = "6A19 C4F0"),
        Device("d-office", "Office laptop", DeviceKind.Laptop, OsPlatform.Windows, trusted = true, fingerprint = "D3B8 0E57"),
    )
    var discovery = DiscoveryState.Idle
    var connection: Connection = Connection.Public
    var name = "Hakim's phone"
    /** The code "the other device" shows, for entering a code. */
    val peerCode = listOf("river", "lantern", "seven")
    private var seq = 0

    private fun emit(e: EngineEvent) {
        _events.tryEmit(e)
    }

    override suspend fun start(name: String?): Snapshot {
        if (name != null) this.name = name
        return Snapshot(discovery, devices, emptyList(), emptyList(), freeSpace = 32_000_000_000)
    }

    override fun stop() {}
    override fun rescan() = emit(EngineEvent.Discovery(DiscoveryState.Searching))

    override suspend fun stage(uris: List<String>) = Staged(uris.map { file(it.substringAfterLast('/'), 2_400_000, "image/jpeg") }, 0)

    override suspend fun send(deviceId: String, files: List<FileItem>): String {
        val t = transfer(TransferDirection.Send, deviceId, files, TransferStatus.Waiting)
        emit(EngineEvent.TransferChanged(t))
        return t.id
    }

    override suspend fun accept(requestId: String, trust: Boolean, autoAccept: Boolean): String = requestId
    override fun decline(requestId: String) {}
    override fun pause(transferId: String) {}
    override fun resume(transferId: String) {}
    override fun cancel(transferId: String) {}
    override fun retry(transferId: String) {}

    override suspend fun createCode(): PairingCode {
        val code = PairingCode(listOf("ember", "marigold", "meadow"), System.currentTimeMillis() + 600_000, "nhakobeam://pair/ember-marigold-meadow")
        emit(EngineEvent.Code(code))
        return code
    }

    override fun cancelCode() = emit(EngineEvent.Code(null))

    override suspend fun redeemCode(words: List<String>): RedeemResult {
        val bad = words.indexOfFirst { it.isBlank() || it.any { c -> !c.isLetter() } }
        if (bad >= 0) return RedeemResult(ok = false, reason = "unknownWord", index = bad)
        if (words.map { it.trim().lowercase() } != peerCode) return RedeemResult(ok = false, reason = "noMatch")
        val d = Device("d-nadia", "Nadia's laptop", DeviceKind.Laptop, OsPlatform.Linux, reachable = true, route = Route.Direct, fingerprint = "8C4E 17B2")
        devices = devices + d
        emit(EngineEvent.DeviceChanged(d))
        emit(EngineEvent.Paired(d, PairVia.Entered, "5D19 E04A"))
        return RedeemResult(ok = true, device = d)
    }

    override fun setTrusted(deviceId: String, trusted: Boolean) = update(deviceId) { it.copy(trusted = trusted, autoAccept = it.autoAccept && trusted) }
    override fun setAutoAccept(deviceId: String, on: Boolean) = update(deviceId) { it.copy(autoAccept = on) }
    override suspend fun renameSelf(name: String) {
        this.name = name
    }

    override suspend fun getSelf() = SelfInfo(name, "/storage/emulated/0/Download/Nhako Beam", "2E7F A0D4", "0.1.0")
    override suspend fun getConnection() = connection
    override suspend fun setConnection(c: Connection) {
        connection = c
    }

    override suspend fun checkServer(c: Connection.Own): ServerCheck {
        val relay = when {
            c.relay.contains("offline") -> CheckResult.Unreachable("nothing answered at that address. Is the server running, and is the port right?")
            c.token.isNullOrBlank() || c.token == "wrong" -> CheckResult.TokenRejected
            else -> CheckResult.Ok(14)
        }
        return ServerCheck(relay, c.lookup?.let { CheckResult.Ok(6) })
    }

    override suspend fun savedFiles(transferId: String) = emptyList<String>()

    private fun update(id: String, change: (Device) -> Device) {
        devices = devices.map { if (it.id == id) change(it).also { d -> emit(EngineEvent.DeviceChanged(d)) } else it }
    }

    // ---------- For tests and previews ----------

    fun file(name: String, size: Long, mime: String) = FileItem("f${++seq}", name, size, mime)

    fun photos() = listOf(
        file("IMG_2041.jpg", 4_200_000, "image/jpeg"),
        file("IMG_2042.jpg", 3_900_000, "image/jpeg"),
        file("IMG_2047.heic", 2_700_000, "image/heic"),
        file("Sunset at Teluk Cempedak.jpg", 5_100_000, "image/jpeg"),
    )

    fun mixed() = listOf(
        file("IMG_3310.jpg", 3_800_000, "image/jpeg"),
        file("IMG_3311.jpg", 4_100_000, "image/jpeg"),
        file("IMG_3312.jpg", 3_600_000, "image/jpeg"),
        file("Wedding highlights final final.mp4", 820_000_000, "video/mp4"),
    )

    fun transfer(direction: TransferDirection, deviceId: String, files: List<FileItem>, status: TransferStatus, done: Double = 0.0): Transfer {
        val total = files.sumOf { it.size }
        val doneBytes = (total * done).toLong()
        var left = doneBytes
        val per = files.associate { f -> f.id to minOf(f.size, left).also { left -= it } }
        val now = System.currentTimeMillis()
        return Transfer(
            id = "t${++seq}", direction = direction, deviceId = deviceId, files = files, totalBytes = total, doneBytes = doneBytes,
            fileBytes = per, speed = if (status == TransferStatus.Transferring) 64_200_000.0 else 0.0,
            route = devices.find { it.id == deviceId }?.route ?: Route.Relayed, status = status, startedAt = now - 20_000,
            movingSince = if (done > 0) now - 12_000 else null, finishedAt = if (status == TransferStatus.Done) now else null,
        )
    }

    fun simTransfer(t: Transfer) = emit(EngineEvent.TransferChanged(t))
    fun simIncoming(deviceId: String = "d-aisyah", files: List<FileItem> = mixed()): IncomingRequest {
        val r = IncomingRequest("r${++seq}", deviceId, files, files.sumOf { it.size }, System.currentTimeMillis())
        emit(EngineEvent.Incoming(r))
        return r
    }
    fun simDiscovery(state: DiscoveryState) = emit(EngineEvent.Discovery(state))
    fun simServer(problem: ServerProblem?) = emit(EngineEvent.Server(problem))
}
