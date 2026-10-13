package io.github.kimzam30.nhakobeam

import android.app.Activity
import android.app.Application
import android.content.ClipData
import android.content.ClipboardManager
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.os.Bundle
import io.github.kimzam30.nhakobeam.engine.RealEngine
import io.github.kimzam30.nhakobeam.platform.Background
import io.github.kimzam30.nhakobeam.platform.Notifier
import io.github.kimzam30.nhakobeam.platform.Push
import io.github.kimzam30.nhakobeam.platform.Sounds
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.SharedPrefs
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

/**
 * One engine and one app state for the whole process, so they outlive screen rotations and keep
 * transfers going while no screen is open.
 */
class NhakoApplication : Application() {
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    lateinit var prefs: SharedPrefs
        private set
    lateinit var app: AppState
        private set
    lateinit var notifier: Notifier
        private set
    lateinit var background: Background
        private set
    lateinit var sounds: Sounds
        private set

    override fun onCreate() {
        super.onCreate()
        prefs = SharedPrefs(this)
        app = AppState(RealEngine(this, scope), prefs, scope)
        notifier = Notifier(this)
        background = Background(this, app, notifier, scope)
        sounds = Sounds(this, prefs) { background.foreground }
        app.sound = sounds::play
        app.afterEvent = background::onEvent
        registerActivityLifecycleCallbacks(Visibility())
        // Text from your own devices is copied even with no screen open; Android lets an app write
        // the clipboard then (only reading needs the app on screen).
        app.clipboard = { text ->
            runCatching { getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("Nhako Beam", text)) }.isSuccess
        }
        watchNetwork()
        app.init()
        Push.start(this)
    }

    /** On Wi-Fi or a cable, nearby devices can be found; on mobile data only your own ones. With no
     *  network at all nothing is claimed (the engine shows that as offline). */
    private fun watchNetwork() {
        val cm = getSystemService(ConnectivityManager::class.java) ?: return
        runCatching {
            cm.registerDefaultNetworkCallback(object : ConnectivityManager.NetworkCallback() {
                override fun onCapabilitiesChanged(network: Network, caps: NetworkCapabilities) {
                    val lan = caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) || caps.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET)
                    scope.launch { app.onLan = lan }
                }

                override fun onLost(network: Network) {
                    scope.launch { app.onLan = true }
                }
            })
        }
    }

    /** Counts our started screens to know whether the app is in sight. */
    private inner class Visibility : ActivityLifecycleCallbacks {
        private var started = 0
        override fun onActivityStarted(activity: Activity) {
            if (started++ == 0) background.foreground = true
        }
        override fun onActivityStopped(activity: Activity) {
            if (--started == 0) background.foreground = false
        }
        override fun onActivityCreated(activity: Activity, savedInstanceState: Bundle?) {}
        override fun onActivityResumed(activity: Activity) {}
        override fun onActivityPaused(activity: Activity) {}
        override fun onActivitySaveInstanceState(activity: Activity, outState: Bundle) {}
        override fun onActivityDestroyed(activity: Activity) {}
    }
}
