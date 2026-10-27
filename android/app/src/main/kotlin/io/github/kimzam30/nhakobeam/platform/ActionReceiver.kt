package io.github.kimzam30.nhakobeam.platform

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import io.github.kimzam30.nhakobeam.NhakoApplication

/** The buttons on our notifications: accept or decline a request, pause, resume or cancel a transfer, let a browser in. */
class ActionReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val nhako = context.applicationContext as NhakoApplication
        val id = intent.getStringExtra(EXTRA_ID) ?: return
        val app = nhako.app
        if (intent.action == STOP_READY) return nhako.background.stopReceiving()
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
            ALLOW_VISITOR, DENY_VISITOR -> {
                app.answerVisitor(id, intent.action == ALLOW_VISITOR)
                nhako.notifier.cancel(Notifier.visitorId(id))
            }
        }
    }

    companion object {
        const val EXTRA_ID = "id"
        const val ACCEPT = "io.github.kimzam30.nhakobeam.ACCEPT"
        const val DECLINE = "io.github.kimzam30.nhakobeam.DECLINE"
        const val PAUSE = "io.github.kimzam30.nhakobeam.PAUSE"
        const val RESUME = "io.github.kimzam30.nhakobeam.RESUME"
        const val CANCEL = "io.github.kimzam30.nhakobeam.CANCEL"
        const val STOP_READY = "io.github.kimzam30.nhakobeam.STOP_READY"
        const val ALLOW_VISITOR = "io.github.kimzam30.nhakobeam.ALLOW_VISITOR"
        const val DENY_VISITOR = "io.github.kimzam30.nhakobeam.DENY_VISITOR"
    }
}
