package io.github.kimzam30.nhakobeam.platform

import io.github.kimzam30.nhakobeam.NhakoApplication

/**
 * The F-Droid build (`-Pnhako.fdroid`): no Firebase, so no wake-up while closed. The phone receives
 * while the app is open or the Receive tile is on, as in any build without push set up.
 * The build with push is in src/push/.
 */
object Push {
    fun start(app: NhakoApplication) {
        app.app.engine.setWakeToken(null)
    }
}
