package io.github.kimzam30.nhakobeam.state

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import io.github.kimzam30.nhakobeam.design.NhakoHaptic
import io.github.kimzam30.nhakobeam.engine.Engine
import io.github.kimzam30.nhakobeam.engine.EngineException
import io.github.kimzam30.nhakobeam.model.Connection
import io.github.kimzam30.nhakobeam.model.Device
import io.github.kimzam30.nhakobeam.model.DiscoveryState
import io.github.kimzam30.nhakobeam.model.EngineEvent
import io.github.kimzam30.nhakobeam.model.FileItem
import io.github.kimzam30.nhakobeam.model.IncomingRequest
import io.github.kimzam30.nhakobeam.model.PairVia
import io.github.kimzam30.nhakobeam.model.PairingCode
import io.github.kimzam30.nhakobeam.model.ServerProblem
import io.github.kimzam30.nhakobeam.model.Snapshot
import io.github.kimzam30.nhakobeam.model.Transfer
import io.github.kimzam30.nhakobeam.model.TransferDirection
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.summarizeFiles
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

enum class Screen { Onboarding, Home, Settings }

sealed interface Sheet {
    data object Send : Sheet
    data object ShowCode : Sheet
    data object EnterCode : Sheet
    data object Scan : Sheet
    data class Paired(val deviceId: String, val via: PairVia, val check: String) : Sheet
    data class TransferDetails(val id: String) : Sheet
    data class ConfirmRemove(val deviceId: String) : Sheet
}

enum class ToastTone { Neutral, Success, Warning, Danger }

data class Toast(val id: Long, val text: String, val tone: ToastTone, val actionLabel: String? = null, val action: (() -> Unit)? = null)

/** What the Android platform does for the app (open files, share, permissions). Tests use the defaults. */
interface PlatformHooks {
    fun openReceived(transfer: Transfer, paths: List<String>) {}
    fun shareReceived(transfer: Transfer, paths: List<String>) {}
    fun haptic(h: NhakoHaptic) {}
    /** Pick files with the system picker; the app gets them back through [AppState.stageUris]. */
    fun pickFiles() {}
}

/**
 * Everything the app shows: what the engine reports plus what the UI is doing. The Kotlin twin of
 * desktop/src/lib/state/app.svelte.ts, so both apps behave the same.
 */
class AppState(val engine: Engine, private val prefs: Prefs, private val scope: CoroutineScope) {
    var platform: PlatformHooks = object : PlatformHooks {}
    /** Called after each engine event has been applied (notifications, the transfer service). */
    var afterEvent: (EngineEvent) -> Unit = {}

    var screen by mutableStateOf(if (prefs.onboarded) Screen.Home else Screen.Onboarding)
    /** 1 when moving forward (deeper), -1 when going back: screens slide the matching way. */
    var navDir by mutableIntStateOf(1)
    var onboarded by mutableStateOf(prefs.onboarded)
    var selfName by mutableStateOf("")
    var saveLocation by mutableStateOf("")
    var fingerprint by mutableStateOf<String?>(null)
    var version by mutableStateOf("")

    var devices by mutableStateOf(listOf<Device>())
    var discovery by mutableStateOf(DiscoveryState.Idle)
    var transfers by mutableStateOf(listOf<Transfer>())
    var requests by mutableStateOf(listOf<IncomingRequest>())
    var freeSpace by mutableLongStateOf(0L)
    var code by mutableStateOf<PairingCode?>(null)
    var staged by mutableStateOf(listOf<FileItem>())
    var sheet by mutableStateOf<Sheet?>(null)
    var toasts by mutableStateOf(listOf<Toast>())
    /** Bumped to make a device orb play its bouncy pulse. */
    var pulse by mutableStateOf(mapOf<String, Int>())
    /** Transfers the user dismissed. */
    var dismissed by mutableStateOf(setOf<String>())
    var connection by mutableStateOf<Connection>(Connection.Public)
    var serverProblem by mutableStateOf<ServerProblem?>(null)
    /** The device tapped before any files were chosen: send to it once they're picked. */
    var pickTarget: String? = null
    /** Reached the engine at least once (the radar shows "starting" before). */
    var started by mutableStateOf(false)

    val reachable by derivedStateOf { devices.filter { it.reachable } }
    /** Radar order: reachable first, trusted first, then by name. */
    val radarDevices by derivedStateOf {
        devices.sortedWith(compareByDescending<Device> { it.reachable }.thenByDescending { it.trusted }.thenBy { it.name })
    }
    val activeTransfers by derivedStateOf { transfers.filter { it.status.isActive } }
    val visibleTransfers by derivedStateOf { transfers.filter { it.id !in dismissed } }
    val stagedSummary by derivedStateOf { summarizeFiles(staged) }

    private var toastSeq = 0L
    /** Devices whose request was accepted on the card, so no extra "Receiving" toast. */
    private val acceptedHere = mutableSetOf<String>()
    private var eventsJob: Job? = null

    /** Listens to the engine, reads the saved settings and, once onboarded, starts the engine. */
    fun init() {
        if (eventsJob != null) return
        eventsJob = scope.launch { engine.events.collect { onEvent(it) } }
        scope.launch {
            runCatching { connection = engine.getConnection() }
            loadSelf()
            if (onboarded) startEngine()
        }
    }

    private suspend fun startEngine(name: String? = null) {
        try {
            applySnapshot(engine.start(name))
            started = true
            loadSelf()
        } catch (e: EngineException) {
            toast(e.message ?: "Couldn't start.", ToastTone.Danger)
            discovery = DiscoveryState.Offline
        }
    }

    private suspend fun loadSelf() {
        runCatching { engine.getSelf() }.onSuccess { me ->
            fingerprint = me.fingerprint
            version = me.version
            saveLocation = me.saveDir
            if (onboarded || selfName.isBlank()) selfName = me.name
        }
    }

    fun onEvent(e: EngineEvent) {
        when (e) {
            is EngineEvent.Discovery -> discovery = e.state
            is EngineEvent.DeviceChanged -> devices = devices.filter { it.id != e.device.id } + e.device
            is EngineEvent.DeviceLost -> devices = devices.filter { it.id != e.deviceId }
            is EngineEvent.Incoming -> if (requests.none { it.id == e.request.id }) requests = requests + e.request
            is EngineEvent.IncomingWithdrawn -> {
                val r = requests.find { it.id == e.requestId }
                requests = requests.filter { it.id != e.requestId }
                if (r != null) toast(t("incoming.withdrawn", "device" to deviceName(r.deviceId)))
            }
            is EngineEvent.TransferChanged -> onTransfer(e.transfer)
            is EngineEvent.Paired -> {
                sheet = Sheet.Paired(e.device.id, e.via, e.check)
                platform.haptic(NhakoHaptic.PairMatched)
            }
            is EngineEvent.Code -> code = e.code
            is EngineEvent.FreeSpace -> freeSpace = e.bytes
            is EngineEvent.Server -> serverProblem = e.problem
            is EngineEvent.SnapshotEvent -> applySnapshot(e.snapshot)
        }
        afterEvent(e)
    }

    /** Redraws from what the engine has. Finished transfers already shown stay as history. */
    fun applySnapshot(s: Snapshot) {
        discovery = s.discovery
        devices = s.devices
        requests = s.requests
        val fresh = s.transfers.associateBy { it.id }
        val kept = transfers.filter { it.id !in fresh && !it.status.isActive }
        transfers = fresh.values.sortedByDescending { it.startedAt } + kept
        code = s.code
        s.freeSpace?.let { freeSpace = it }
        serverProblem = s.serverProblem
    }

    private fun onTransfer(next: Transfer) {
        val prev = transfers.find { it.id == next.id }
        if (prev != null) {
            transfers = transfers.map { if (it.id == next.id) next else it }
        } else {
            transfers = listOf(next) + transfers
            if (next.direction == TransferDirection.Receive && !acceptedHere.remove(next.deviceId)) {
                val d = device(next.deviceId)
                if (d?.autoAccept == true) toast(t("incoming.autoAccepted", "device" to d.name), ToastTone.Success)
            }
        }
        if (prev?.status == next.status) return
        when {
            next.status == TransferStatus.Done -> {
                platform.haptic(NhakoHaptic.TransferDone)
                bump(next.deviceId)
            }
            next.status == TransferStatus.Failed -> platform.haptic(NhakoHaptic.TransferFailed)
            next.status == TransferStatus.Declined -> platform.haptic(NhakoHaptic.Decline)
            prev?.status == TransferStatus.Reconnecting && next.status == TransferStatus.Transferring ->
                toast(t("transfer.reconnected"), ToastTone.Success)
        }
    }

    // ---------- Navigation ----------

    fun navigate(to: Screen) {
        navDir = if (to.ordinal >= screen.ordinal) 1 else -1
        screen = to
    }

    fun finishOnboarding() {
        onboarded = true
        prefs.onboarded = true
        navigate(Screen.Home)
        scope.launch { startEngine(selfName.trim().ifEmpty { null }) }
    }

    fun deviceName(id: String) = device(id)?.name ?: "The other device"
    fun device(id: String) = devices.find { it.id == id }
    fun transferFor(deviceId: String) = activeTransfers.find { it.deviceId == deviceId }

    fun bump(deviceId: String) {
        pulse = pulse + (deviceId to (pulse[deviceId] ?: 0) + 1)
    }

    // ---------- Toasts ----------

    fun toast(text: String, tone: ToastTone = ToastTone.Neutral, actionLabel: String? = null, action: (() -> Unit)? = null) {
        val toast = Toast(++toastSeq, text, tone, actionLabel, action)
        toasts = (toasts + toast).takeLast(2)
        scope.launch {
            delay(if (action != null) 6000 else 4000)
            dismissToast(toast.id)
        }
    }

    fun dismissToast(id: Long) {
        toasts = toasts.filter { it.id != id }
    }

    // ---------- Sending ----------

    /** Files picked in the system picker or shared from another app. */
    fun stageUris(uris: List<String>, openSendSheet: Boolean = false) {
        if (uris.isEmpty()) return
        scope.launch {
            val picked = runCatching { engine.stage(uris) }.getOrElse {
                toast(it.message ?: t("send.failed"), ToastTone.Danger)
                return@launch
            }
            if (picked.skipped > 0) {
                toast(if (picked.skipped == 1) t("send.skipped.one") else t("send.skipped.many", "n" to picked.skipped), ToastTone.Warning)
            }
            stage(picked.files)
            val target = pickTarget
            pickTarget = null
            if (target != null && picked.files.isNotEmpty()) {
                sendTo(target)
                return@launch
            }
            if (openSendSheet && picked.files.isNotEmpty() && onboarded) {
                if (screen != Screen.Home) navigate(Screen.Home)
                if (sheet == null || sheet == Sheet.Send) sheet = Sheet.Send
            }
        }
    }

    fun stage(files: List<FileItem>) {
        if (files.isNotEmpty()) staged = staged + files
    }

    fun clearStaged() {
        staged = emptyList()
    }

    fun sendTo(deviceId: String, files: List<FileItem> = staged) {
        if (files.isEmpty()) return
        val sending = files.toList()
        staged = emptyList()
        bump(deviceId)
        if (sheet == Sheet.Send) sheet = null
        scope.launch {
            try {
                engine.send(deviceId, sending)
            } catch (e: Exception) {
                if (staged.isEmpty()) staged = sending
                toast(e.message ?: t("send.failed"), ToastTone.Danger)
            }
        }
    }

    // ---------- Receiving ----------

    fun accept(requestId: String, autoAccept: Boolean = false) {
        val r = requests.find { it.id == requestId }
        requests = requests.filter { it.id != requestId }
        if (r != null) {
            acceptedHere += r.deviceId
            bump(r.deviceId)
        }
        scope.launch {
            runCatching { engine.accept(requestId, trust = autoAccept, autoAccept = autoAccept) }
                .onFailure { toast(it.message ?: "Couldn't accept.", ToastTone.Danger) }
        }
    }

    fun decline(requestId: String) {
        requests = requests.filter { it.id != requestId }
        engine.decline(requestId)
    }

    fun dismissTransfer(id: String) {
        dismissed = dismissed + id
        if (sheet == Sheet.TransferDetails(id)) sheet = null
    }

    fun openReceived(transfer: Transfer, share: Boolean = false) {
        scope.launch {
            val paths = runCatching { engine.savedFiles(transfer.id) }.getOrDefault(emptyList())
            if (paths.isEmpty()) {
                toast(t("transfer.openFailed"), ToastTone.Danger)
                return@launch
            }
            if (share) platform.shareReceived(transfer, paths) else platform.openReceived(transfer, paths)
        }
    }

    // ---------- Codes and settings ----------

    fun newCode() {
        scope.launch {
            runCatching { engine.createCode() }.onFailure { toast(it.message ?: "Couldn't make a code.", ToastTone.Danger) }
        }
    }

    fun renameSelf(name: String) {
        selfName = name
        scope.launch { runCatching { engine.renameSelf(name) } }
    }

    /** Saves the server setting; the engine reconnects through it. False (with a toast) if it couldn't. */
    suspend fun setConnection(c: Connection): Boolean {
        try {
            engine.setConnection(c)
        } catch (e: EngineException) {
            toast(e.message ?: "Couldn't save.", ToastTone.Danger)
            return false
        }
        connection = c
        toast(t(if (c is Connection.Own) "settings.connection.saved.own" else "settings.connection.saved.public"), ToastTone.Success)
        return true
    }
}
