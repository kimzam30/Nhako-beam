package io.github.kimzam30.nhakobeam.platform

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import io.github.kimzam30.nhakobeam.NhakoApplication

/** The buttons on our notifications: accept or decline a request, pause, resume or cancel a transfer. */
class ActionReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val nhako = context.applicationContext as NhakoApplication
        val id = intent.getStringExtra(EXTRA_ID) ?: return
        val app = nhako.app
        when (intent.action) {
            ACCEPT -> {
                app.accept(id)
                nhako.notifier.cancel(Notifier.requestId(id))
            }
            DECLINE -> {
                app.decline(id)
                nhako.notifier.cancel(Notifier.requestId(id))
            }
            PAUSE -> app.engine.pause(id)
            RESUME -> app.engine.resume(id)
            CANCEL -> app.engine.cancel(id)
        }
    }

    companion object {
        const val EXTRA_ID = "id"
        const val ACCEPT = "io.github.kimzam30.nhakobeam.ACCEPT"
        const val DECLINE = "io.github.kimzam30.nhakobeam.DECLINE"
        const val PAUSE = "io.github.kimzam30.nhakobeam.PAUSE"
        const val RESUME = "io.github.kimzam30.nhakobeam.RESUME"
        const val CANCEL = "io.github.kimzam30.nhakobeam.CANCEL"
    }
}
