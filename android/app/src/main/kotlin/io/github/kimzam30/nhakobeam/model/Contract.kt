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
    /** Not reachable right now, but can be woken up to receive (a phone Android put to sleep). */
    val wakeable: Boolean = false,
)

@Serializable
data class FileItem(
    val id: String,
    val name: String,
    val size: Long,
    val mime: String = "application/octet-stream",
    /** The folder it's in, inside a folder that was sent ("Photos/2024"). Null for files sent on their own. */
    val folder: String? = null,
)

/** The longest text sent as text, in bytes of UTF-8. Longer text goes as a file. */
const val TEXT_MAX = 64 * 1024

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
    /** The receiver only takes files from its own devices (Who can send to me). */
    @SerialName("notAllowed") NotAllowed,
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
    /** Sender: the other device was asleep and has been asked to wake up. */
    val waking: Boolean = false,
    /** A text transfer (a note, a link, a clipboard): the text. */
    val text: String? = null,
)

/** A transfer that ended, as History lists it. Kept after a restart, and after the device is removed. */
@Serializable
data class HistoryEntry(
    val id: String,
    val direction: TransferDirection,
    val deviceId: String,
    val deviceName: String,
    val deviceKind: DeviceKind = DeviceKind.Laptop,
    val files: List<FileItem>,
    val totalBytes: Long,
    val doneBytes: Long = 0,
    val route: Route = Route.Relayed,
    /** Done, failed, cancelled or declined. */
    val status: TransferStatus,
    val error: TransferError? = null,
    val cancelledBy: Side? = null,
    val startedAt: Long = 0,
    val finishedAt: Long,
    /** Bytes per second while bytes were moving (pauses left out). 0 when nothing moved. */
    val averageSpeed: Double = 0.0,
    /** A text transfer: the text. */
    val text: String? = null,
)

@Serializable
data class IncomingRequest(
    val id: String,
    val deviceId: String,
    val files: List<FileItem>,
    val totalBytes: Long,
    val receivedAt: Long = 0,
    /** Text rather than files: the text, to read before accepting. */
    val text: String? = null,
)

/** Another device on the same network asking to be one of your devices, and you one of its own
 *  (Trust this device, from Nearby). Both screens show [check]. */
@Serializable
data class TrustRequest(val deviceId: String, val check: String, val receivedAt: Long = 0)

/** How a trust ask ended without both devices trusting each other. */
@Serializable
enum class TrustOutcome {
    @SerialName("declined") Declined,
    @SerialName("withdrawn") Withdrawn,
    @SerialName("outdated") Outdated,
    @SerialName("unreachable") Unreachable,
    @SerialName("busy") Busy,
}

/** A newer Nhako Beam on GitHub (Settings > About > Check for new versions). */
@Serializable
data class UpdateInfo(
    /** "0.3.0". */
    val version: String,
    /** Its release page, where the downloads are. */
    val url: String,
)

/** Files (and or text) shared with a link, for a device without the app (Share via link). */
@Serializable
data class LinkShare(
    val id: String,
    /** "http://192.168.1.20:41234/k3jq.../", also in the QR code. */
    val url: String,
    val files: List<FileItem> = emptyList(),
    val totalBytes: Long = 0,
    val text: String? = null,
    /** Epoch ms. Downloads running then carry on, and the link stops after them. */
    val expiresAt: Long,
    /** Browsers that opened the link, in the order they came. */
    val visitors: List<LinkVisitor> = emptyList(),
)

/** A browser that opened a shared link. */
@Serializable
data class LinkVisitor(
    val id: String,
    /** "Chrome on Android", from what the browser says about itself. */
    val label: String,
    /** Its address on the network, "192.168.1.23". */
    val address: String,
    val state: VisitorState,
    val arrivedAt: Long = 0,
    /** Downloads running now: bytes so far, and their sizes. Both 0 when nothing is running. */
    val activeBytes: Long = 0,
    val activeTotal: Long = 0,
    /** Downloads finished (a file, or the zip of everything). */
    val downloads: Int = 0,
)

@Serializable
enum class VisitorState {
    /** Waiting for you to let it in. */
    @SerialName("asking") Asking,
    @SerialName("allowed") Allowed,
    @SerialName("denied") Denied,
}

/** Four words (since Phase 11). */
@Serializable
data class PairingCode(val words: List<String>, val expiresAt: Long, val uri: String)

@Serializable
enum class DiscoveryState {
    @SerialName("searching") Searching,
    @SerialName("idle") Idle,
    @SerialName("blocked") Blocked,
    @SerialName("offline") Offline,
}

/** Settings > Receiving > Who can send to me. Trusted devices, and devices connected with a code, can always send. */
@Serializable
enum class WhoCanSend(val wire: String) {
    @SerialName("yourDevices") YourDevices("yourDevices"),
    @SerialName("everyoneNearby") EveryoneNearby("everyoneNearby");

    companion object {
        fun fromWire(s: String) = entries.firstOrNull { it.wire == s } ?: YourDevices
    }
}

@Serializable
enum class PairVia {
    @SerialName("shown") Shown,
    @SerialName("entered") Entered,
    /** Trust this device, from Nearby, agreed on both. */
    @SerialName("nearby") Nearby,
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
    /** Newest first. */
    val history: List<HistoryEntry> = emptyList(),
    /** Trust asks waiting for an answer here. */
    val trustAsks: List<TrustRequest> = emptyList(),
    /** The link being shared, if any. */
    val linkShare: LinkShare? = null,
    /** A newer version, when the last check found one. */
    val update: UpdateInfo? = null,
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

    /** A transfer moved: only what changed since its last `transfer` or `transferProgress` event, to merge in. */
    @Serializable @SerialName("transferProgress")
    data class TransferProgress(
        val id: String,
        val doneBytes: Long,
        val speed: Double = 0.0,
        val movingSince: Long? = null,
        /** Only the files whose count changed. */
        val fileBytes: Map<String, Long> = emptyMap(),
    ) : EngineEvent

    @Serializable @SerialName("paired")
    data class Paired(val device: Device, val via: PairVia, val check: String) : EngineEvent

    @Serializable @SerialName("code")
    data class Code(val code: PairingCode? = null) : EngineEvent

    @Serializable @SerialName("freeSpace")
    data class FreeSpace(val bytes: Long) : EngineEvent

    @Serializable @SerialName("server")
    data class Server(val problem: ServerProblem? = null) : EngineEvent

    /** A transfer ended and was added to History (or replaced its own entry after a retry). */
    @Serializable @SerialName("history")
    data class History(val entry: HistoryEntry) : EngineEvent

    /** A device nearby asks to trust this one, and this one it. */
    @Serializable @SerialName("trustAsk")
    data class TrustAsk(val request: TrustRequest) : EngineEvent

    /** On the device that asked: the other device has the ask, and both screens show [check]. */
    @Serializable @SerialName("trustWaiting")
    data class TrustWaiting(val deviceId: String, val check: String) : EngineEvent

    /** A trust ask ended without a match (on either device). A match ends with [Paired] instead. */
    @Serializable @SerialName("trustEnded")
    data class TrustEnded(val deviceId: String, val outcome: TrustOutcome) : EngineEvent

    /** The link being shared changed: started, a browser came or moved, or it stopped (null). */
    @Serializable @SerialName("linkShare")
    data class LinkShareChanged(val share: LinkShare? = null) : EngineEvent

    /** A newer version is out (or, null, none is). */
    @Serializable @SerialName("update")
    data class Update(val update: UpdateInfo? = null) : EngineEvent

    @Serializable @SerialName("snapshot")
    data class SnapshotEvent(val snapshot: Snapshot) : EngineEvent
}
