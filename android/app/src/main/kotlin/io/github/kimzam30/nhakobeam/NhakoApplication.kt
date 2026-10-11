package io.github.kimzam30.nhakobeam

import android.app.Activity
import android.app.Application
import android.os.Bundle
import io.github.kimzam30.nhakobeam.engine.RealEngine
import io.github.kimzam30.nhakobeam.platform.Background
import io.github.kimzam30.nhakobeam.platform.Notifier
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.SharedPrefs
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob

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

    override fun onCreate() {
        super.onCreate()
        prefs = SharedPrefs(this)
        app = AppState(RealEngine(this, scope), prefs, scope)
        notifier = Notifier(this)
        background = Background(this, app, notifier, scope)
        app.afterEvent = background::onEvent
        registerActivityLifecycleCallbacks(Visibility())
        app.init()
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
