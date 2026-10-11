// The engine contract, the same JSON as desktop/src/lib/engine/types.ts (and core/src/types.rs).
// The engine sends this JSON across the FFI; kotlinx.serialization reads it here.
@file:OptIn(ExperimentalSerializationApi::class)

package io.github.kimzam30.nhakobeam.model

import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonClassDiscriminator

/** One JSON setup for everything the engine sends. Unknown fields are skipped (newer engines). */
val ContractJson = Json {
    ignoreUnknownKeys = true
    explicitNulls = false
    encodeDefaults = true
}

@Serializable
enum class DeviceKind {
    @SerialName("laptop") Laptop,
    @SerialName("desktop") Desktop,
    @SerialName("phone") Phone,
    @SerialName("tablet") Tablet,
}

@Serializable
enum class OsPlatform {
    @SerialName("windows") Windows,
    @SerialName("linux") Linux,
    @SerialName("android") Android,
    @SerialName("macos") Macos,
    @SerialName("ios") Ios,
    @SerialName("unknown") Unknown,
}

/** How bytes travel: same network, direct over the internet, or through a relay. */
@Serializable
enum class Route {
    @SerialName("nearby") Nearby,
    @SerialName("direct") Direct,
    @SerialName("relayed") Relayed,
}

@Serializable
data class Device(
    val id: String,
    val name: String,
    val kind: DeviceKind,
    val os: OsPlatform = OsPlatform.Unknown,
    val trusted: Boolean = false,
    val autoAccept: Boolean = false,
    val reachable: Boolean = false,
    val route: Route? = null,
    val fingerprint: String = "",
)

@Serializable
data class FileItem(val id: String, val name: String, val size: Long, val mime: String = "application/octet-stream")

@Serializable
enum class TransferDirection {
    @SerialName("send") Send,
    @SerialName("receive") Receive,
}

@Serializable
enum class TransferStatus {
    @SerialName("waiting") Waiting,
    @SerialName("connecting") Connecting,
    @SerialName("transferring") Transferring,
    @SerialName("paused") Paused,
    @SerialName("reconnecting") Reconnecting,
    @SerialName("done") Done,
    @SerialName("failed") Failed,
    @SerialName("cancelled") Cancelled,
    @SerialName("declined") Declined;

    val isActive: Boolean get() = this in setOf(Waiting, Connecting, Transferring, Paused, Reconnecting)
}

@Serializable
enum class TransferError {
    @SerialName("peerGone") PeerGone,
    @SerialName("network") Network,
    @SerialName("fileChanged") FileChanged,
    @SerialName("integrity") Integrity,
    @SerialName("relayUnreachable") RelayUnreachable,
    @SerialName("permission") Permission,
    @SerialName("diskFull") DiskFull,
    @SerialName("diskFullPeer") DiskFullPeer,
    @SerialName("unknown") Unknown,
}

@Serializable
enum class Side {
    @SerialName("me") Me,
    @SerialName("peer") Peer,
}

@Serializable
data class Transfer(
    val id: String,
    val direction: TransferDirection,
    val deviceId: String,
    val files: List<FileItem>,
    val totalBytes: Long,
    val doneBytes: Long = 0,
    val fileBytes: Map<String, Long> = emptyMap(),
    val speed: Double = 0.0,
    val route: Route = Route.Relayed,
    val status: TransferStatus,
    val pausedBy: Side? = null,
    val cancelledBy: Side? = null,
    val error: TransferError? = null,
    val startedAt: Long = 0,
    val movingSince: Long? = null,
    val finishedAt: Long? = null,
    val waitingLong: Boolean = false,
)

@Serializable
data class IncomingRequest(val id: String, val deviceId: String, val files: List<FileItem>, val totalBytes: Long, val receivedAt: Long = 0)

@Serializable
data class PairingCode(val words: List<String>, val expiresAt: Long, val uri: String)

@Serializable
enum class DiscoveryState {
    @SerialName("searching") Searching,
    @SerialName("idle") Idle,
    @SerialName("blocked") Blocked,
    @SerialName("offline") Offline,
}

@Serializable
enum class PairVia {
    @SerialName("shown") Shown,
    @SerialName("entered") Entered,
}

@Serializable
enum class ServerProblem {
    @SerialName("unreachable") Unreachable,
    @SerialName("tokenRejected") TokenRejected,
}

/** `{ok: true, device}` or `{ok: false, reason, index?}`. */
@Serializable
data class RedeemResult(val ok: Boolean, val device: Device? = null, val reason: String? = null, val index: Int? = null)

@Serializable
data class Snapshot(
    val discovery: DiscoveryState,
    val devices: List<Device>,
    val transfers: List<Transfer>,
    val requests: List<IncomingRequest>,
    val code: PairingCode? = null,
    val freeSpace: Long? = null,
    val serverProblem: ServerProblem? = null,
)

@Serializable
data class SelfInfo(val name: String, val saveDir: String, val fingerprint: String? = null, val version: String = "")

/** Which servers help devices find and reach each other over the internet. */
@Serializable
@JsonClassDiscriminator("mode")
sealed interface Connection {
    @Serializable @SerialName("public")
    data object Public : Connection

    @Serializable @SerialName("own")
    data class Own(val relay: String, val token: String? = null, val lookup: String? = null) : Connection
}

@Serializable
@JsonClassDiscriminator("status")
sealed interface CheckResult {
    @Serializable @SerialName("ok")
    data class Ok(val ms: Int) : CheckResult

    @Serializable @SerialName("unreachable")
    data class Unreachable(val reason: String) : CheckResult

    @Serializable @SerialName("tokenRejected")
    data object TokenRejected : CheckResult

    @Serializable @SerialName("invalid")
    data class Invalid(val reason: String) : CheckResult
}

@Serializable
data class ServerCheck(val relay: CheckResult, val lookup: CheckResult? = null)

@Serializable
@JsonClassDiscriminator("type")
sealed interface EngineEvent {
    @Serializable @SerialName("discovery")
    data class Discovery(val state: DiscoveryState) : EngineEvent

    @Serializable @SerialName("device")
    data class DeviceChanged(val device: Device) : EngineEvent

    @Serializable @SerialName("deviceLost")
    data class DeviceLost(val deviceId: String) : EngineEvent

    @Serializable @SerialName("incoming")
    data class Incoming(val request: IncomingRequest) : EngineEvent

    @Serializable @SerialName("incomingWithdrawn")
    data class IncomingWithdrawn(val requestId: String) : EngineEvent

    @Serializable @SerialName("transfer")
    data class TransferChanged(val transfer: Transfer) : EngineEvent

    @Serializable @SerialName("paired")
    data class Paired(val device: Device, val via: PairVia, val check: String) : EngineEvent

    @Serializable @SerialName("code")
    data class Code(val code: PairingCode? = null) : EngineEvent

    @Serializable @SerialName("freeSpace")
    data class FreeSpace(val bytes: Long) : EngineEvent

    @Serializable @SerialName("server")
    data class Server(val problem: ServerProblem? = null) : EngineEvent

    @Serializable @SerialName("snapshot")
    data class SnapshotEvent(val snapshot: Snapshot) : EngineEvent
}
