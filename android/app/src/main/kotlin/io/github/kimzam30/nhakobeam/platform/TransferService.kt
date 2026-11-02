package io.github.kimzam30.nhakobeam.platform

import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.ServiceCompat
import io.github.kimzam30.nhakobeam.NhakoApplication

/**
 * Keeps the app alive while files are moving, so a transfer carries on with the screen off or the
 * app in the background, and for the few minutes it's kept ready to receive (the Quick Settings
 * tile, or a wake-up from another device). It only shows the notification; [Background] starts
 * and stops it.
 */
class TransferService : Service() {
    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val nhako = application as NhakoApplication
        val background = nhako.background
        val type = if (Build.VERSION.SDK_INT >= 29) ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC else 0
        val notification = background.serviceNotification()
        // Android stops the whole app if a service started this way never goes to the foreground, or
        // is stopped before it does. So it always does, even when a send already ended (refused in
        // a few milliseconds), and only then steps aside.
        try {
            ServiceCompat.startForeground(this, Notifier.PROGRESS_ID, notification ?: nhako.notifier.always(), type)
        } catch (e: RuntimeException) {
            // Android refused (its daily time for this kind of service ran out). The transfer still
            // runs while Android keeps us around; the next one tries again.
            android.util.Log.w("nhako", "Couldn't show the transfer notification: ${e.message}")
            background.serviceStopped()
            stopSelf()
            return START_NOT_STICKY
        }
        background.serviceStarted()
        if (notification == null) {
            ServiceCompat.stopForeground(this, ServiceCompat.STOP_FOREGROUND_REMOVE)
            background.serviceStopped()
            stopSelf()
        }
        return START_NOT_STICKY
    }

    /** Android 15 limits data sync services to 6 hours a day; when that runs out, step aside. */
    override fun onTimeout(startId: Int, fgsType: Int) {
        (application as NhakoApplication).background.serviceStopped()
        stopSelf()
    }
}
