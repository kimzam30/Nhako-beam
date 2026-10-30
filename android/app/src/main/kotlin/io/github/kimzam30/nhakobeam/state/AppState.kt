package io.github.kimzam30.nhakobeam.state

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import io.github.kimzam30.nhakobeam.design.NhakoHaptic
import io.github.kimzam30.nhakobeam.design.NhakoSound
import io.github.kimzam30.nhakobeam.engine.Engine
import io.github.kimzam30.nhakobeam.engine.EngineException
import io.github.kimzam30.nhakobeam.model.Connection
import io.github.kimzam30.nhakobeam.model.Device
import io.github.kimzam30.nhakobeam.model.DiscoveryState
import io.github.kimzam30.nhakobeam.model.EngineEvent
import io.github.kimzam30.nhakobeam.model.FileItem
import io.github.kimzam30.nhakobeam.model.HistoryEntry
import io.github.kimzam30.nhakobeam.model.IncomingRequest
import io.github.kimzam30.nhakobeam.model.LinkShare
import io.github.kimzam30.nhakobeam.model.PairVia
import io.github.kimzam30.nhakobeam.model.PairingCode
import io.github.kimzam30.nhakobeam.model.ServerProblem
import io.github.kimzam30.nhakobeam.model.Side
import io.github.kimzam30.nhakobeam.model.Snapshot
import io.github.kimzam30.nhakobeam.model.TEXT_MAX
import io.github.kimzam30.nhakobeam.model.Transfer
import io.github.kimzam30.nhakobeam.model.TransferDirection
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.model.TrustOutcome
import io.github.kimzam30.nhakobeam.model.TrustRequest
import io.github.kimzam30.nhakobeam.model.UpdateInfo
import io.github.kimzam30.nhakobeam.model.VisitorState
import io.github.kimzam30.nhakobeam.model.WhoCanSend
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.isLink
import io.github.kimzam30.nhakobeam.ui.isRunnable
import io.github.kimzam30.nhakobeam.ui.summarizeFiles
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/** In order of depth: going to a later one slides forward, an earlier one back. */
enum class Screen { Onboarding, Home, History, Settings, Licenses }

sealed interface Sheet {
    data object Send : Sheet
    data object ShowCode : Sheet
    data object EnterCode : Sheet
    data object Scan : Sheet
    data class Paired(val deviceId: String, val via: PairVia, val check: String) : Sheet
    data class TransferDetails(val id: String) : Sheet
    data class ConfirmRemove(val deviceId: String) : Sheet
    data class HistoryItem(val id: String) : Sheet
    data object ConfirmClearHistory : Sheet
    /** Opening a received program or script: asks first. [past] is a History entry rather than a transfer card. */
    data class ConfirmOpen(val id: String, val fileName: String, val deviceName: String, val past: Boolean) : Sheet
    /** Type or paste text to send; to [deviceId] straight away when it's set. */
    data class Compose(val deviceId: String? = null, val text: String? = null) : Sheet
    /** A device's own sheet (long press on it): send, trust. */
    data class DeviceSheet(val deviceId: String) : Sheet
    /** This device asked another to trust it; [check] arrives once the other device has the ask. */
    data class TrustWait(val deviceId: String, val check: String? = null) : Sheet
    /** Another device asks to trust this one. */
    data class TrustAsk(val deviceId: String, val check: String) : Sheet
    /** Text that was sent or received (a transfer, or a History entry). */
    data class TextSheet(val id: String) : Sheet
    /** The link being shared: its QR, who opened it, Stop. */
    data object Link : Sheet
}

enum class ToastTone { Neutral, Success, Warning, Danger }

/** [ms] is how long it stays; its time-left bar drains over it. */
data class Moments(val found: Long = 0, val done: Long = 0, val failed: Long = 0)

data class Toast(val id: Long, val text: String, val tone: ToastTone, val actionLabel: String? = null, val action: (() -> Unit)? = null, val ms: Long = 4000)

/** What the Android platform does for the app (open files, share, permissions). Tests use the defaults. */
interface PlatformHooks {
    /** Opens received files (several: their folder in the Files app). */
    fun openFiles(paths: List<String>) {}
    fun shareFiles(paths: List<String>) {}
    fun haptic(h: NhakoHaptic) {}
    /** Pick files with the system picker; the app gets them back through [AppState.stageUris]. */
    fun pickFiles() {}
    /** Pick a folder with the system picker; the app gets it back through [AppState.stageFolder]. */
    fun pickFolder() {}
    /** The clipboard's text ("" when there's none). Android only lets an app on screen read it. */
    fun readClipboard(): String = ""
    /** Puts text on the clipboard. False if it couldn't. */
    fun writeClipboard(text: String): Boolean = false
    fun openLink(url: String) {}
    /** Android lets the app run in the background as it likes (no battery limits on it). */
    fun backgroundAllowed(): Boolean = true
    /** Asks Android to stop limiting the app in the background (one system prompt). */
    fun allowBackground() {}
}

/**
 * Everything the app shows: what the engine reports plus what the UI is doing. The Kotlin twin of
 * desktop/src/lib/state/app.svelte.ts, so both apps behave the same.
 */
class AppState(val engine: Engine, private val prefs: Prefs, private val scope: CoroutineScope) {
    var platform: PlatformHooks = object : PlatformHooks {}
    /** Called after each engine event has been applied (notifications, the transfer service). */
    var afterEvent: (EngineEvent) -> Unit = {}
    /** Plays one of the app's sounds (platform/Sounds.kt decides whether it's heard). */
    var sound: (NhakoSound) -> Unit = {}
    /** Puts text on the clipboard, with a screen open or not. Tests keep the screen's own. */
    var clipboard: (String) -> Boolean = { platform.writeClipboard(it) }

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
    /** Text waiting to be sent, instead of files (the tray shows it; a device takes it). */
    var stagedText by mutableStateOf<String?>(null)
    /** On Wi-Fi or a cable, where nearby devices can be found (from the system's network callback). */
    var onLan by mutableStateOf(true)
    /** Trust asks from other devices, waiting for an answer here. */
    var trustAsks by mutableStateOf(listOf<TrustRequest>())
    private var sheetState by mutableStateOf<Sheet?>(null)
    /** The open sheet. Sliding one up or away has a sound; the paired sheet has its own (Matched). */
    var sheet: Sheet?
        get() = sheetState
        set(v) {
            val was = sheetState
            sheetState = v
            if (v != null && v !is Sheet.Paired && v::class != was?.let { it::class }) sound(NhakoSound.Open)
            else if (v == null && was != null) sound(NhakoSound.Close)
        }
    var toasts by mutableStateOf(listOf<Toast>())
    /** Bumped to make a device orb play its bouncy pulse. */
    var pulse by mutableStateOf(mapOf<String, Int>())
    /** Transfers the user dismissed. */
    var dismissed by mutableStateOf(setOf<String>())
    var connection by mutableStateOf<Connection>(Connection.Public)
    var serverProblem by mutableStateOf<ServerProblem?>(null)
    /** Finished transfers, newest first (kept by the engine across restarts). */
    var history by mutableStateOf(listOf<HistoryEntry>())
    /** Announce this device and look for others on the local network (saved by the engine). */
    var lanDiscovery by mutableStateOf(true)
    /** Who can send to me (saved by the engine). */
    var whoCanSend by mutableStateOf(WhoCanSend.YourDevices)
    /** Other devices can wake this phone while the app is closed (push is set up and has a token). */
    var wakeReady by mutableStateOf(false)
    /** The device tapped before any files were chosen: send to it once they're picked. */
    var pickTarget: String? = null
    /** Reached the engine at least once (the radar shows "starting" before). */
    var started by mutableStateOf(false)
    /** The link being shared (Share via link). */
    var linkShare by mutableStateOf<LinkShare?>(null)
    /** Browsers already heard asking, so each one rings once. */
    private val visitorsHeard = mutableSetOf<String>()
    /** A newer version, when the daily check found one. */
    var update by mutableStateOf<UpdateInfo?>(null)
    /** Check for new versions (saved by the engine); null in builds that get updates from a store. */
    var updateCheck by mutableStateOf<Boolean?>(null)
    /** The version whose Home banner was closed with Not now. */
    var updateSeen by mutableStateOf(prefs.updateSeen)
    /** Always ready to receive, with no screen open (Android: platform/ReceiverService.kt). */
    var alwaysReady by mutableStateOf(prefs.alwaysReady)
    /** Starts or stops always ready to receive to match [onboarded] and [alwaysReady]. */
    var syncAlways: () -> Unit = {}
    /** Sends being handed to the engine. Files shared from another app can only be opened until
     *  the quick send sheet closes, so it waits for this to reach 0. */
    var sending by mutableIntStateOf(0)
    /** When something worth a reaction last happened (ms): the mascot shows it for a moment
     *  (stage/MascotMood.kt). Set where the matching sounds play. */
    var moments by mutableStateOf(Moments())

    val reachable by derivedStateOf { devices.filter { it.reachable } }
    /** Radar order: reachable first, trusted first, then by name. */
    val radarDevices by derivedStateOf {
        devices.sortedWith(compareByDescending<Device> { it.reachable }.thenByDescending { it.trusted }.thenBy { it.name })
    }
    val activeTransfers by derivedStateOf { transfers.filter { it.status.isActive } }
    val visibleTransfers by derivedStateOf { transfers.filter { it.id !in dismissed } }
    val stagedSummary by derivedStateOf { summarizeFiles(staged) }
    /** Something is waiting to be sent: files or text. */
    val hasStaged by derivedStateOf { staged.isNotEmpty() || stagedText != null }

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
            runCatching { lanDiscovery = engine.getLanDiscovery() }
            runCatching { whoCanSend = engine.getWhoCanSend() }
            runCatching { updateCheck = engine.getUpdateCheck() }
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
            is EngineEvent.DeviceChanged -> {
                val was = device(e.device.id)
                devices = devices.filter { it.id != e.device.id } + e.device
                if (e.device.reachable && was?.reachable != true) foundSound()
            }
            is EngineEvent.DeviceLost -> devices = devices.filter { it.id != e.deviceId }
            is EngineEvent.Incoming -> if (requests.none { it.id == e.request.id }) {
                requests = requests + e.request
                sound(NhakoSound.Incoming)
            }
            is EngineEvent.IncomingWithdrawn -> {
                val r = requests.find { it.id == e.requestId }
                requests = requests.filter { it.id != e.requestId }
                if (r != null) {
                    toast(t("incoming.withdrawn", "device" to deviceName(r.deviceId)))
                    sound(NhakoSound.Cancel)
                }
            }
            is EngineEvent.TransferChanged -> onTransfer(e.transfer)
            is EngineEvent.TransferProgress -> transfers = transfers.map {
                if (it.id != e.id) it
                else it.copy(doneBytes = e.doneBytes, speed = e.speed, movingSince = e.movingSince, fileBytes = if (e.fileBytes.isEmpty()) it.fileBytes else it.fileBytes + e.fileBytes)
            }
            is EngineEvent.Paired -> {
                trustAsks = trustAsks.filter { it.deviceId != e.device.id }
                sheet = Sheet.Paired(e.device.id, e.via, e.check)
                platform.haptic(NhakoHaptic.PairMatched)
                sound(NhakoSound.Matched)
                moments = moments.copy(found = System.currentTimeMillis())
            }
            is EngineEvent.Code -> code = e.code
            is EngineEvent.FreeSpace -> freeSpace = e.bytes
            is EngineEvent.Server -> serverProblem = e.problem
            is EngineEvent.History -> history = (listOf(e.entry) + history.filter { it.id != e.entry.id }).sortedByDescending { it.finishedAt }
            is EngineEvent.TrustAsk -> {
                trustAsks = trustAsks.filter { it.deviceId != e.request.deviceId } + e.request
                // Someone is holding the other device and waiting: this comes first.
                sheet = Sheet.TrustAsk(e.request.deviceId, e.request.check)
                sound(NhakoSound.Incoming)
            }
            is EngineEvent.TrustWaiting -> (sheet as? Sheet.TrustWait)?.let { if (it.deviceId == e.deviceId) sheet = it.copy(check = e.check) }
            is EngineEvent.TrustEnded -> trustEnded(e.deviceId, e.outcome)
            is EngineEvent.LinkShareChanged -> onLinkShare(e.share)
            is EngineEvent.Update -> update = e.update
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
        history = s.history
        trustAsks = s.trustAsks
        linkShare = s.linkShare
        s.linkShare?.visitors?.forEach { visitorsHeard += it.id }
        update = s.update
    }

    private fun onTransfer(next: Transfer) {
        val prev = transfers.find { it.id == next.id }
        if (prev != null) {
            transfers = transfers.map { if (it.id == next.id) next else it }
        } else {
            transfers = listOf(next) + transfers
            if (next.direction == TransferDirection.Receive && !acceptedHere.remove(next.deviceId)) {
                val d = device(next.deviceId)
                if (d?.autoAccept == true && next.text != null) textArrived(next, d.name)
                else if (d?.autoAccept == true) {
                    toast(t("incoming.autoAccepted", "device" to d.name), ToastTone.Success)
                    sound(NhakoSound.Incoming)
                }
            }
        }
        if (prev?.status == next.status) return
        when {
            next.status == TransferStatus.Done -> {
                platform.haptic(NhakoHaptic.TransferDone)
                sound(NhakoSound.Done)
                moments = moments.copy(done = System.currentTimeMillis())
                bump(next.deviceId)
            }
            next.status == TransferStatus.Failed -> {
                platform.haptic(NhakoHaptic.TransferFailed)
                sound(NhakoSound.Failed)
                moments = moments.copy(failed = System.currentTimeMillis())
            }
            next.status == TransferStatus.Declined -> {
                platform.haptic(NhakoHaptic.Decline)
                sound(NhakoSound.Decline)
            }
            next.status == TransferStatus.Cancelled && next.cancelledBy == Side.Peer -> sound(NhakoSound.Cancel)
            prev?.status == TransferStatus.Reconnecting && next.status == TransferStatus.Transferring ->
                toast(t("transfer.reconnected"), ToastTone.Success)
        }
    }

    // ---------- Navigation ----------

    fun navigate(to: Screen) {
        navDir = if (to.ordinal >= screen.ordinal) 1 else -1
        if (to != screen && to != Screen.Onboarding && screen != Screen.Onboarding) sound(if (navDir > 0) NhakoSound.Open else NhakoSound.Close)
        screen = to
    }

    /** Devices arriving all at once (just after launch, or back on Wi-Fi) make one sound, not many. */
    private var foundQuietUntil = System.currentTimeMillis() + 2500
    private fun foundSound() {
        val now = System.currentTimeMillis()
        if (now < foundQuietUntil) return
        foundQuietUntil = now + 2000
        sound(NhakoSound.Found)
        moments = moments.copy(found = now)
    }

    fun finishOnboarding() {
        onboarded = true
        prefs.onboarded = true
        navigate(Screen.Home)
        scope.launch { startEngine(selfName.trim().ifEmpty { null }) }
        syncAlways()
    }

    fun changeAlwaysReady(on: Boolean) {
        alwaysReady = on
        prefs.alwaysReady = on
        syncAlways()
    }

    fun deviceName(id: String) = device(id)?.name ?: "The other device"
    fun device(id: String) = devices.find { it.id == id }
    fun transferFor(deviceId: String) = activeTransfers.find { it.deviceId == deviceId }

    fun bump(deviceId: String) {
        pulse = pulse + (deviceId to (pulse[deviceId] ?: 0) + 1)
    }

    // ---------- Toasts ----------

    fun toast(text: String, tone: ToastTone = ToastTone.Neutral, actionLabel: String? = null, action: (() -> Unit)? = null) {
        val toast = Toast(++toastSeq, text, tone, actionLabel, action, if (action != null) 6000 else 4000)
        if (tone == ToastTone.Danger) sound(NhakoSound.Error)
        toasts = (toasts + toast).takeLast(2)
        scope.launch {
            delay(toast.ms)
            dismissToast(toast.id)
        }
    }

    fun dismissToast(id: Long) {
        toasts = toasts.filter { it.id != id }
    }

    // ---------- Text ----------

    /** Text from one of your own devices with "Accept files automatically" on goes straight to the clipboard. */
    private fun textArrived(tr: Transfer, device: String) {
        val text = tr.text.orEmpty()
        clipboard(text)
        val link = isLink(text)
        if (link) toast(t("text.copied.link", "device" to device), ToastTone.Success, t("text.open")) { openLink(text) }
        else toast(t("text.copied.from", "device" to device), ToastTone.Success, t("text.show")) { sheet = Sheet.TextSheet(tr.id) }
    }

    /** Copies text you chose to copy (a button), with a toast. */
    fun copyText(text: String): Boolean {
        if (!clipboard(text)) {
            toast(t("text.copyFailed"), ToastTone.Danger)
            return false
        }
        platform.haptic(NhakoHaptic.ButtonPress)
        toast(t("text.copied"), ToastTone.Success)
        return true
    }

    fun openLink(url: String) {
        if (isLink(url)) platform.openLink(url.trim())
    }

    /** Text to send instead of files. False (with a toast) when it's empty or too long. */
    fun stageText(text: String): Boolean {
        if (text.isBlank()) {
            toast(t("send.clipboard.empty"), ToastTone.Warning)
            return false
        }
        if (text.toByteArray().size > TEXT_MAX) {
            toast(t("send.text.tooLong"), ToastTone.Warning)
            return false
        }
        staged = emptyList()
        stagedText = text
        return true
    }

    /** Home > Clipboard: whatever text is on the clipboard, ready to go to a device. */
    fun stageClipboard() = stageText(platform.readClipboard())

    /** Text shared from another app (a link from the browser): ready to go, and pick a device. */
    fun shareText(text: String) {
        if (!stageText(text) || !onboarded) return
        if (screen != Screen.Home) navigate(Screen.Home)
        if (sheet == null || sheet == Sheet.Send) sheet = Sheet.Send
    }

    fun sendTextTo(deviceId: String, text: String) {
        stagedText = null
        bump(deviceId)
        if (sheet == Sheet.Send || sheet is Sheet.Compose) sheet = null
        sound(NhakoSound.Send)
        sending++
        scope.launch {
            try {
                engine.sendText(deviceId, text)
            } catch (e: Exception) {
                if (stagedText == null && staged.isEmpty()) stagedText = text
                toast(e.message ?: t("send.failed"), ToastTone.Danger)
            } finally {
                sending--
            }
        }
    }

    // ---------- Share via link ----------

    /** Shares what's waiting (files or text) with a link, for a device without the app. */
    fun shareLink() {
        if (!hasStaged) return
        val files = staged
        val text = stagedText
        scope.launch {
            try {
                linkShare = engine.shareLink(files, text)
                staged = emptyList()
                stagedText = null
                sheet = Sheet.Link
                sound(NhakoSound.Send)
            } catch (e: Exception) {
                toast(e.message ?: t("send.failed"), ToastTone.Danger)
            }
        }
    }

    fun stopLink() {
        engine.stopLink()
        linkShare = null
        if (sheet == Sheet.Link) sheet = null
        toast(t("link.stopped"))
        sound(NhakoSound.Cancel)
    }

    fun answerVisitor(visitorId: String, allow: Boolean) {
        linkShare = linkShare?.let { share ->
            share.copy(visitors = share.visitors.map { if (it.id == visitorId) it.copy(state = if (allow) VisitorState.Allowed else VisitorState.Denied) else it })
        }
        engine.answerVisitor(visitorId, allow)
        platform.haptic(if (allow) NhakoHaptic.Accept else NhakoHaptic.Decline)
        sound(if (allow) NhakoSound.Accept else NhakoSound.Decline)
    }

    private fun onLinkShare(share: LinkShare?) {
        val had = linkShare
        linkShare = share
        if (share == null) {
            visitorsHeard.clear()
            if (sheet == Sheet.Link) sheet = null
            // Stopped here says so already; this is the link ending by itself.
            if (had != null) toast(t("link.ended"))
            return
        }
        for (v in share.visitors) {
            if (v.state != VisitorState.Asking || !visitorsHeard.add(v.id)) continue
            sound(NhakoSound.Incoming)
            // Someone is standing there with a browser: show who, unless something else is open.
            if (sheet == null) sheet = Sheet.Link
            else if (sheet != Sheet.Link) toast(t("link.asks", "browser" to v.label), ToastTone.Neutral, t("text.show")) { sheet = Sheet.Link }
        }
    }

    // ---------- Updates ----------

    suspend fun setUpdateCheck(on: Boolean): Boolean {
        try {
            engine.setUpdateCheck(on)
        } catch (e: Exception) {
            toast(t("settings.about.updates.failed"), ToastTone.Danger)
            return false
        }
        updateCheck = on
        if (!on) update = null
        return true
    }

    /** The Home banner's Not now: hidden until the next version. */
    fun hideUpdate() {
        val u = update ?: return
        updateSeen = u.version
        prefs.updateSeen = u.version
    }

    /** Opens the release page, where the downloads are. */
    fun openUpdate() {
        update?.let { openLink(it.url) }
    }

    // ---------- Trust from Nearby ----------

    /** Asks a nearby device to trust this one, and to be trusted back; the sheet waits for it. */
    fun askTrust(deviceId: String) {
        sheet = Sheet.TrustWait(deviceId)
        scope.launch {
            try {
                engine.askTrust(deviceId)
            } catch (e: Exception) {
                if ((sheet as? Sheet.TrustWait)?.deviceId == deviceId) sheet = null
                toast(e.message ?: t("send.failed"), ToastTone.Danger)
            }
        }
    }

    fun cancelTrust(deviceId: String) {
        engine.cancelTrust(deviceId)
        if ((sheet as? Sheet.TrustWait)?.deviceId == deviceId) sheet = null
    }

    fun answerTrust(deviceId: String, yes: Boolean) {
        trustAsks = trustAsks.filter { it.deviceId != deviceId }
        if ((sheet as? Sheet.TrustAsk)?.deviceId == deviceId) sheet = null
        engine.answerTrust(deviceId, yes)
        // A yes opens the paired sheet when both have saved each other.
    }

    private fun trustEnded(deviceId: String, outcome: TrustOutcome) {
        trustAsks = trustAsks.filter { it.deviceId != deviceId }
        val open = sheet.let { (it is Sheet.TrustWait && it.deviceId == deviceId) || (it is Sheet.TrustAsk && it.deviceId == deviceId) }
        val asking = sheet is Sheet.TrustWait
        if (open) sheet = null
        // Saying no here needs no toast; everything else says what happened.
        if (outcome == TrustOutcome.Declined && !asking) return
        if (!open && outcome != TrustOutcome.Withdrawn) return
        val key = "trust.ended." + outcome.name.replaceFirstChar { it.lowercase() }
        val quiet = outcome == TrustOutcome.Declined || outcome == TrustOutcome.Withdrawn
        toast(t(key, "device" to deviceName(deviceId)), if (quiet) ToastTone.Neutral else ToastTone.Warning)
        sound(if (outcome == TrustOutcome.Declined) NhakoSound.Decline else NhakoSound.Cancel)
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

    /** A folder picked in the system picker: everything inside it, keeping its shape. */
    fun stageFolder(treeUri: String) {
        scope.launch {
            val picked = runCatching { engine.stageFolder(treeUri) }.getOrElse {
                pickTarget = null
                toast(it.message ?: t("send.failed"), ToastTone.Danger)
                return@launch
            }
            if (picked.skipped > 0) {
                toast(if (picked.skipped == 1) t("send.skipped.one") else t("send.skipped.many", "n" to picked.skipped), ToastTone.Warning)
            }
            stage(picked.files)
            val target = pickTarget
            pickTarget = null
            if (target != null && picked.files.isNotEmpty()) sendTo(target)
        }
    }

    fun stage(files: List<FileItem>) {
        if (files.isEmpty()) return
        stagedText = null
        staged = staged + files
    }

    fun clearStaged() {
        staged = emptyList()
        stagedText = null
    }

    fun sendTo(deviceId: String, files: List<FileItem> = staged) {
        val text = stagedText
        if (text != null && files === staged) return sendTextTo(deviceId, text)
        if (files.isEmpty()) return
        val sending = files.toList()
        staged = emptyList()
        bump(deviceId)
        if (sheet == Sheet.Send) sheet = null
        sound(NhakoSound.Send)
        this.sending++
        scope.launch {
            try {
                engine.send(deviceId, sending)
            } catch (e: Exception) {
                if (staged.isEmpty()) staged = sending
                toast(e.message ?: t("send.failed"), ToastTone.Danger)
            } finally {
                this@AppState.sending--
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

    /** Opens or shares what a transfer received. A program or script (opening it runs it) asks first; [confirmed] is the answer. */
    fun openReceived(transfer: Transfer, share: Boolean = false, confirmed: Boolean = false) {
        val only = transfer.files.singleOrNull()
        if (!share && !confirmed && only != null && isRunnable(only.name)) {
            sheet = Sheet.ConfirmOpen(transfer.id, only.name, device(transfer.deviceId)?.name ?: "another device", past = false)
            return
        }
        if (confirmed && sheet is Sheet.ConfirmOpen) sheet = null
        scope.launch {
            val paths = runCatching { engine.savedFiles(transfer.id) }.getOrDefault(emptyList())
            if (paths.isEmpty()) {
                toast(t("transfer.openFailed"), ToastTone.Danger)
                return@launch
            }
            if (share) platform.shareFiles(paths) else platform.openFiles(paths)
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

    /** Turns nearby search on or off; the engine reconnects. False (with a toast) if it couldn't. */
    suspend fun setLanDiscovery(on: Boolean): Boolean {
        try {
            engine.setLanDiscovery(on)
        } catch (e: EngineException) {
            toast(e.message ?: "Couldn't save.", ToastTone.Danger)
            return false
        }
        lanDiscovery = on
        toast(t(if (on) "settings.network.lan.on" else "settings.network.lan.off"), ToastTone.Success)
        return true
    }

    /** Settings > About: copies the diagnostics report with [clip]. False (with a toast) if it couldn't. */
    suspend fun copyDiagnostics(clip: (String) -> Unit): Boolean {
        try {
            clip(engine.diagnostics())
        } catch (e: Exception) {
            toast(t("settings.about.diagnostics.failed"), ToastTone.Danger)
            return false
        }
        toast(t("settings.about.diagnostics.copied"), ToastTone.Success)
        return true
    }

    /** Settings > Receiving > Who can send to me. False (with a toast) if it couldn't be saved. */
    suspend fun setWhoCanSend(who: WhoCanSend): Boolean {
        val before = whoCanSend
        whoCanSend = who
        try {
            engine.setWhoCanSend(who)
        } catch (e: Exception) {
            whoCanSend = before
            toast(t("settings.receive.who.failed"), ToastTone.Danger)
            return false
        }
        return true
    }

    fun clearHistory() {
        engine.clearHistory()
        history = emptyList()
        sheet = null
        toast(t("history.cleared"), ToastTone.Success)
    }

    fun historyEntry(id: String) = history.find { it.id == id }

    /** Opens or shares what a History entry received (the engine remembers where it saved them). */
    fun openPast(entry: HistoryEntry, share: Boolean = false, confirmed: Boolean = false) {
        val only = entry.files.singleOrNull()
        if (!share && !confirmed && only != null && isRunnable(only.name)) {
            sheet = Sheet.ConfirmOpen(entry.id, only.name, device(entry.deviceId)?.name ?: entry.deviceName, past = true)
            return
        }
        if (confirmed && sheet is Sheet.ConfirmOpen) sheet = null
        scope.launch {
            val paths = runCatching { engine.savedFiles(entry.id) }.getOrDefault(emptyList())
            if (paths.isEmpty()) {
                toast(t("transfer.openFailed"), ToastTone.Danger)
                return@launch
            }
            if (share) platform.shareFiles(paths) else platform.openFiles(paths)
        }
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
