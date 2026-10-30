package io.github.kimzam30.nhakobeam.platform

import android.app.PendingIntent
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import io.github.kimzam30.nhakobeam.MainActivity
import io.github.kimzam30.nhakobeam.NhakoApplication
import io.github.kimzam30.nhakobeam.ui.Copy.t

/**
 * The "Receive" Quick Settings tile: one tap keeps this phone ready to receive for 5 minutes
 * without opening the app (nearby devices can find it, requests come as notifications). Tap again
 * to stop. With Always ready to receive on, it shows "Always on" and pauses or resumes that
 * instead. docs/design/screens-and-states.md, section 10.3.
 */
class ReceiveTile : TileService() {
    private val nhako get() = application as NhakoApplication

    override fun onStartListening() {
        nhako.background.onReadyChanged = ::refresh
        refresh()
    }

    override fun onStopListening() {
        val app = nhako
        app.background.onReadyChanged = { update(app) }
    }

    override fun onClick() {
        val background = nhako.background
        when {
            // Not set up yet: the app has to be opened once first.
            !nhako.app.onboarded -> openApp()
            // Always ready to receive: the tile pauses it, and a second tap brings it back.
            background.alwaysOn -> background.pauseAlways()
            background.ready -> background.stopReceiving()
            nhako.app.alwaysReady -> background.resumeAlways()
            else -> background.receiveFor(READY_MS, woken = false)
        }
        refresh()
    }

    private fun refresh() {
        val tile = qsTile ?: return
        val background = nhako.background
        tile.state = if (background.alwaysOn || background.ready) Tile.STATE_ACTIVE else Tile.STATE_INACTIVE
        tile.label = t("tile.receive")
        if (Build.VERSION.SDK_INT >= 29) {
            tile.subtitle = when {
                background.alwaysOn -> t("tile.receive.always")
                background.ready -> t("tile.receive.on")
                else -> null
            }
        }
        tile.updateTile()
    }

    private fun openApp() {
        val intent = Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        if (Build.VERSION.SDK_INT >= 34) {
            startActivityAndCollapse(PendingIntent.getActivity(this, 0, intent, PendingIntent.FLAG_IMMUTABLE))
        } else {
            @Suppress("DEPRECATION")
            startActivityAndCollapse(intent)
        }
    }

    companion object {
        const val READY_MS = 5 * 60_000L

        /** Asks the system to redraw the tile (when the app changes it from elsewhere). */
        fun update(context: Context) {
            runCatching { requestListeningState(context, ComponentName(context, ReceiveTile::class.java)) }
        }
    }
}
