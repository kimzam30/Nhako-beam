package io.github.kimzam30.nhakobeam.platform

import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.ServiceCompat
import io.github.kimzam30.nhakobeam.NhakoApplication

/**
 * Always ready to receive (Settings > Receiving, on by default): keeps the app running with no
 * screen open, from the moment the phone starts ([BootReceiver]), so your devices can send to it
 * at any time, as if the app were open. It holds a quiet notification with Pause; [Background]
 * starts and stops it.
 *
 * Its type is "connected device" (talking to your other devices over the network), which Android
 * lets start at boot and doesn't stop after 6 hours a day, unlike "data sync". Transfers still get
 * [TransferService] and its progress notification.
 */
class ReceiverService : Service() {
    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val nhako = application as NhakoApplication
        val background = nhako.background
        val type = if (Build.VERSION.SDK_INT >= 29) ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE else 0
        return try {
            // Android stops an app whose service was started this way and never went to the
            // foreground, so it does even when it's about to stop.
            ServiceCompat.startForeground(this, Notifier.ALWAYS_ID, nhako.notifier.always(), type)
            if (!background.alwaysWanted) {
                ServiceCompat.stopForeground(this, ServiceCompat.STOP_FOREGROUND_REMOVE)
                background.alwaysStopped()
                stopSelf()
                return START_NOT_STICKY
            }
            background.alwaysStarted()
            // Android may stop the app for memory; this brings it back, and the engine with it.
            START_STICKY
        } catch (e: RuntimeException) {
            // Started from the background where Android doesn't allow it (it allows boot, an app
            // update, the tile and the screen). The next of those tries again.
            android.util.Log.w("nhako", "Couldn't stay ready to receive: ${e.message}")
            background.alwaysStopped()
            stopSelf()
            START_NOT_STICKY
        }
    }
}
