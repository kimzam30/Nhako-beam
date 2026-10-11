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
    private var lastProgress = 0L
    private var lastStatus: TransferStatus? = null
    private val postedRequests = mutableSetOf<String>()
    private val finished = mutableSetOf<String>()

    private val multicast: WifiManager.MulticastLock? by lazy {
        (context.applicationContext.getSystemService(Context.WIFI_SERVICE) as? WifiManager)
            ?.createMulticastLock("nhako-discovery")
            ?.apply { setReferenceCounted(false) }
    }

    fun onEvent(e: EngineEvent) {
        when (e) {
            is EngineEvent.Incoming -> if (!foreground) {
                notifier.showRequest(e.request, app.deviceName(e.request.deviceId))
                postedRequests += e.request.id
            }
            is EngineEvent.TransferChanged -> onTransfer(e)
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
        scope.launch {
            val paths = runCatching { app.engine.savedFiles(tr.id) }.getOrDefault(emptyList())
            if (paths.isNotEmpty()) MediaScannerConnection.scanFile(context, paths.toTypedArray(), null, null)
        }
        if (!foreground) notifier.showReceived(tr, app.deviceName(tr.deviceId))
    }

    fun progressNotification(): Notification? {
        val active = app.activeTransfers
        return if (active.isEmpty()) null else notifier.progress(active, app::deviceName)
    }

    /** Starts the service when files start moving, keeps its notification fresh, stops it after. */
    private fun syncService() {
        val active = app.activeTransfers
        if (active.isEmpty()) {
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
        val status = active.first().status
        if (status != lastStatus || now - lastProgress >= 1000) {
            lastStatus = status
            lastProgress = now
            progressNotification()?.let { notifier.updateProgress(it) }
        }
    }

    /**
     * Phones drop multicast packets to save power unless an app holds this lock. Hold it while the
     * app is on screen or files are moving, so nearby devices show up and stay connected.
     */
    private fun updateMulticast() {
        val want = foreground || app.activeTransfers.isNotEmpty()
        val lock = multicast ?: return
        runCatching {
            if (want && !lock.isHeld) lock.acquire() else if (!want && lock.isHeld) lock.release()
        }
    }
}
