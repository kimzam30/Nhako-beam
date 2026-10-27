package io.github.kimzam30.nhakobeam.platform

import android.app.Notification
import android.content.Context
import android.content.Intent
import android.media.MediaScannerConnection
import android.net.wifi.WifiManager
import android.os.SystemClock
import android.util.Log
import androidx.core.content.ContextCompat
import io.github.kimzam30.nhakobeam.model.EngineEvent
import io.github.kimzam30.nhakobeam.model.TransferDirection
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.state.AppState
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * What happens around the screen: the transfer service and its notification, request and "got
 * files" notifications while the app is out of sight, the Wi-Fi multicast lock that lets nearby
 * devices find each other, and the media scan that puts received photos in the Gallery.
 */
class Background(private val context: Context, private val app: AppState, private val notifier: Notifier, private val scope: CoroutineScope) {
    /** True while one of our screens is showing. */
    var foreground = false
        set(v) {
            field = v
            if (v) {
                // The incoming card shows the requests now.
                postedRequests.forEach { notifier.cancel(Notifier.requestId(it)) }
                postedRequests.clear()
            }
            updateMulticast()
        }

    private var serviceRunning = false
    /** Ready to receive until this moment (elapsed realtime), with no screen open. 0: not. */
    private var readyUntil = 0L
    /** Woken up by another device (rather than the tile). */
    private var readyWoken = false
    private var readyJob: Job? = null
    /** The Quick Settings tile listens while it's showing, to switch on or off. */
    var onReadyChanged: () -> Unit = { ReceiveTile.update(context) }

    val ready: Boolean get() = readyUntil > SystemClock.elapsedRealtime()

    /**
     * Keeps the phone reachable for [ms] with no screen open: the engine runs, nearby devices can
     * find it, and requests show as notifications. [woken]: another device asked (a push), so it
     * only lasts long enough for that device to connect.
     */
    fun receiveFor(ms: Long, woken: Boolean) {
        if (woken && foreground) return
        readyUntil = SystemClock.elapsedRealtime() + ms
        readyWoken = woken
        readyJob?.cancel()
        readyJob = scope.launch {
            // The notification counts the minutes down, then it stops by itself.
            while (ready) {
                delay(minOf(30_000L, readyUntil - SystemClock.elapsedRealtime()).coerceAtLeast(1L))
                if (ready) syncService()
            }
            stopReceiving()
        }
        if (app.started) app.engine.rescan()
        lastStatus = null
        syncService()
        updateMulticast()
        onReadyChanged()
    }

    fun stopReceiving() {
        readyUntil = 0L
        readyJob?.cancel()
        readyJob = null
        lastStatus = null
        syncService()
        updateMulticast()
        onReadyChanged()
    }
    private var lastProgress = 0L
    private var lastStatus: TransferStatus? = null
    private val postedRequests = mutableSetOf<String>()
    private val finished = mutableSetOf<String>()

    private val multicast: WifiManager.MulticastLock? by lazy {
        (context.applicationContext.getSystemService(Context.WIFI_SERVICE) as? WifiManager)
            ?.createMulticastLock("nhako-discovery")
            ?.apply { setReferenceCounted(false) }
    }

    /** Browsers asking at a shared link, each with its notification (in the background). */
    private val postedVisitors = mutableSetOf<String>()

    private fun onLinkShare(e: EngineEvent.LinkShareChanged) {
        val asking = e.share?.visitors.orEmpty().filter { it.state == io.github.kimzam30.nhakobeam.model.VisitorState.Asking }
        if (!foreground) for (v in asking) if (postedVisitors.add(v.id)) notifier.showVisitor(v)
        // Answered on screen, or the link stopped: their notifications go.
        val still = asking.map { it.id }.toSet()
        postedVisitors.filter { it !in still }.forEach {
            notifier.cancel(Notifier.visitorId(it))
            postedVisitors -= it
        }
    }

    fun onEvent(e: EngineEvent) {
        when (e) {
            is EngineEvent.Incoming -> if (!foreground) {
                notifier.showRequest(e.request, app.deviceName(e.request.deviceId))
                postedRequests += e.request.id
            }
            is EngineEvent.TransferChanged -> onTransfer(e)
            is EngineEvent.TrustAsk -> if (!foreground) notifier.showTrustAsk(e.request.deviceId, app.deviceName(e.request.deviceId))
            is EngineEvent.TrustEnded -> notifier.cancel(Notifier.trustId(e.deviceId))
            is EngineEvent.Paired -> notifier.cancel(Notifier.trustId(e.device.id))
            is EngineEvent.LinkShareChanged -> onLinkShare(e)
            else -> {}
        }
        // Requests that were answered, withdrawn or timed out lose their notification.
        val open = app.requests.map { it.id }.toSet()
        postedRequests.filter { it !in open }.forEach {
            notifier.cancel(Notifier.requestId(it))
            postedRequests -= it
        }
        syncService()
    }

    private fun onTransfer(e: EngineEvent.TransferChanged) {
        val tr = e.transfer
        if (tr.status != TransferStatus.Done || tr.direction != TransferDirection.Receive || !finished.add(tr.id)) return
        if (tr.text == null) scope.launch {
            val paths = runCatching { app.engine.savedFiles(tr.id) }.getOrDefault(emptyList())
            if (paths.isNotEmpty()) MediaScannerConnection.scanFile(context, paths.toTypedArray(), null, null)
        }
        if (foreground) return
        // Text: copied straight away from your own devices; from anyone else it was accepted on screen.
        if (tr.text != null) {
            if (app.device(tr.deviceId)?.autoAccept == true) notifier.showCopied(tr, app.deviceName(tr.deviceId))
        } else notifier.showReceived(tr, app.deviceName(tr.deviceId))
    }

    fun progressNotification(): Notification? {
        val active = app.activeTransfers
        return if (active.isEmpty()) null else notifier.progress(active, app::deviceName)
    }

    /** What the foreground service shows: the transfers, or that it's ready to receive. */
    fun serviceNotification(): Notification? = progressNotification() ?: readyNotification()

    private fun readyNotification(): Notification? {
        if (!ready) return null
        val minutes = ((readyUntil - SystemClock.elapsedRealtime() + 59_999) / 60_000).toInt()
        return notifier.ready(readyWoken, minutes)
    }

    /** Starts the service when files start moving, keeps its notification fresh, stops it after. */
    private fun syncService() {
        val active = app.activeTransfers
        if (active.isEmpty() && !ready) {
            if (serviceRunning) {
                context.stopService(Intent(context, TransferService::class.java))
                serviceRunning = false
            }
            notifier.cancel(Notifier.PROGRESS_ID)
            lastStatus = null
            updateMulticast()
            return
        }
        if (!serviceRunning) {
            try {
                ContextCompat.startForegroundService(context, Intent(context, TransferService::class.java))
                serviceRunning = true
            } catch (e: IllegalStateException) {
                // Android won't let a background app start one (a request accepted automatically
                // while out of sight). The transfer still runs while Android keeps us around.
                Log.w("nhako", "Couldn't start the transfer service: ${e.message}")
            }
            updateMulticast()
        }
        // Progress moves many times a second; the notification follows once a second, or at once
        // when the state changes (paused, resumed).
        val now = SystemClock.elapsedRealtime()
        val status = active.firstOrNull()?.status
        if (status != lastStatus || now - lastProgress >= 1000) {
            lastStatus = status
            lastProgress = now
            serviceNotification()?.let { notifier.updateProgress(it) }
        }
    }

    /**
     * Phones drop multicast packets to save power unless an app holds this lock. Hold it while the
     * app is on screen or files are moving, so nearby devices show up and stay connected.
     */
    private fun updateMulticast() {
        val want = foreground || ready || app.activeTransfers.isNotEmpty()
        val lock = multicast ?: return
        runCatching {
            if (want && !lock.isHeld) lock.acquire() else if (!want && lock.isHeld) lock.release()
        }
    }
}
