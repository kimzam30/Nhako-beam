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
 * app in the background. It only shows the progress notification; [Background] starts and stops it.
 */
class TransferService : Service() {
    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val notification = (application as NhakoApplication).background.progressNotification()
        if (notification == null) {
            stopSelf()
            return START_NOT_STICKY
        }
        val type = if (Build.VERSION.SDK_INT >= 29) ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC else 0
        ServiceCompat.startForeground(this, Notifier.PROGRESS_ID, notification, type)
        return START_NOT_STICKY
    }

    /** Android 15 limits data sync services to 6 hours a day; when that runs out, step aside. */
    override fun onTimeout(startId: Int, fgsType: Int) {
        stopSelf()
    }
}
