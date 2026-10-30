package io.github.kimzam30.nhakobeam.platform

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import io.github.kimzam30.nhakobeam.NhakoApplication

/**
 * The phone started, or the app was updated: start receiving again without the app being opened
 * (only once it's set up, and with Always ready to receive on). Starting the app starts the engine
 * ([NhakoApplication]); this starts [ReceiverService], which keeps it running.
 */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED && intent.action != Intent.ACTION_MY_PACKAGE_REPLACED) return
        (context.applicationContext as NhakoApplication).background.syncAlways()
    }
}
