package io.github.kimzam30.nhakobeam.platform

import android.util.Log
import com.google.firebase.FirebaseApp
import com.google.firebase.messaging.FirebaseMessaging
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import io.github.kimzam30.nhakobeam.NhakoApplication
import kotlinx.coroutines.launch

/**
 * Wake-up while closed (wake/README.md): this phone gets a Firebase Cloud Messaging token, the
 * engine shares it with trusted devices, and when one of them can't reach the phone it asks the
 * wake service to push an empty "wake up" to it.
 *
 * Only active in a build with the project's google-services.json, on a phone with Google Play
 * services. Otherwise nothing here runs and the phone receives while the app is open, as before.
 */
object Push {
    /** Registers for pushes; Firebase hands the token to [WakeMessagingService.onRegistered]. */
    fun start(app: NhakoApplication) {
        if (FirebaseApp.getApps(app).isEmpty()) {
            app.app.engine.setWakeToken(null)
            return
        }
        runCatching {
            FirebaseMessaging.getInstance().register()
                .addOnFailureListener { Log.i("nhako", "No push token, so this phone can't be woken: ${it.message}") }
        }.onFailure { Log.i("nhako", "Push isn't available: ${it.message}") }
    }

    /** Firebase may call from its own threads; the app's state lives on the main one. */
    fun remember(app: NhakoApplication, token: String?) {
        app.scope.launch {
            app.app.engine.setWakeToken(token)
            app.app.wakeReady = token != null
        }
    }
}

/** Firebase calls this with the token (when it's new or changes) and with wake-up pushes. */
class WakeMessagingService : FirebaseMessagingService() {
    private val nhako get() = application as NhakoApplication

    override fun onRegistered(token: String) {
        Push.remember(nhako, token)
    }

    override fun onUnregistered(token: String) {
        Push.remember(nhako, null)
    }

    override fun onMessageReceived(message: RemoteMessage) {
        if (message.data["nhako"] != "wake") return
        // The app object starting up has already started the engine (once set up). Keep it
        // reachable long enough for the sender, who keeps trying for about 45 seconds.
        nhako.scope.launch { nhako.background.receiveFor(WAKE_MS, woken = true) }
    }

    companion object {
        const val WAKE_MS = 90_000L
    }
}
